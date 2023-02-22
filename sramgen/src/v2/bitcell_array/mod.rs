use arcstr::ArcStr;

use serde::{Deserialize, Serialize};
use substrate::component::Component;

pub mod cbl;
pub mod layout;
pub mod replica;
pub mod schematic;

pub struct SpCellArray {
    params: SpCellArrayParams,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct SpCellArrayParams {
    pub rows: usize,
    pub cols: usize,
    pub mux_ratio: usize,
}

impl Component for SpCellArray {
    type Params = SpCellArrayParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        if params.rows % 8 != 0 || params.cols % 8 != 0 || params.rows == 0 || params.cols == 0 {
            return Err(substrate::component::error::Error::InvalidParams.into());
        }
        Ok(Self { params: *params })
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

#[cfg(test)]
mod tests {
    use substrate::component::{Component, NoParams};
    use substrate::layout::cell::Port;
    use substrate::layout::elements::via::{Via, ViaParams};
    use substrate::layout::geom::{Dir, Rect, Shape, Side, Sides, Span};
    use substrate::layout::layers::selector::Selector;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use crate::v2::bitcell_array::layout::*;
    use crate::v2::guard_ring::{GuardRingParams, GuardRingWrapper, WrapperParams};

    use super::*;

    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct SpCellArrayWithGuardRingParams {
        inner: SpCellArrayParams,
        h_width: i64,
        v_width: i64,
    }

    pub struct SpCellArrayWithGuardRing {
        params: SpCellArrayWithGuardRingParams,
    }

    impl Component for SpCellArrayWithGuardRing {
        type Params = SpCellArrayWithGuardRingParams;

        fn new(
            params: &Self::Params,
            _ctx: &substrate::data::SubstrateCtx,
        ) -> substrate::error::Result<Self> {
            Ok(Self {
                params: params.clone(),
            })
        }

        fn name(&self) -> ArcStr {
            arcstr::literal!("sp_cell_array_with_guard_ring")
        }

        fn schematic(
            &self,
            ctx: &mut substrate::schematic::context::SchematicCtx,
        ) -> substrate::error::Result<()> {
            let mut array = ctx.instantiate::<SpCellArray>(&self.params.inner)?;
            ctx.bubble_all_ports(&mut array);
            ctx.add_instance(array);
            Ok(())
        }

        fn layout(
            &self,
            ctx: &mut substrate::layout::context::LayoutCtx,
        ) -> substrate::error::Result<()> {
            let v_metal = ctx.layers().get(Selector::Metal(1))?;
            let h_metal = ctx.layers().get(Selector::Metal(2))?;
            let m3 = ctx.layers().get(Selector::Metal(3))?;
            let params: WrapperParams<SpCellArrayParams> = WrapperParams {
                inner: self.params.inner,
                enclosure: 2_000,
                h_metal,
                v_metal,
                h_width: self.params.h_width,
                v_width: self.params.v_width,
            };
            let array = ctx.instantiate::<GuardRingWrapper<SpCellArray>>(&params)?;

            let top_limit = array
                .port("ring_vss")?
                .first_rect(h_metal, Side::Top)?
                .bottom();

            for (ring_port_name, side, port_names) in [
                ("ring_vss", Side::Left, vec!["vgnd", "wl_dummy"]),
                ("ring_vdd", Side::Left, vec!["vpwr"]),
                ("ring_vss", Side::Top, vec!["vgnd"]),
                ("ring_vdd", Side::Top, vec!["vpwr", "bl_dummy", "br_dummy"]),
            ] {
                for port_name in port_names {
                    let ring_port = array.port(ring_port_name)?;
                    let port = array.port(port_name)?;
                    let dir = side.coord_dir();
                    let (ring_metal, port_metal) = match dir {
                        Dir::Horiz => (v_metal, h_metal),
                        Dir::Vert => (h_metal, v_metal),
                    };

                    let start = port.first_rect(port_metal, side)?.side(side);

                    for shape in port.shapes(port_metal) {
                        if let Shape::Rect(r) = shape {
                            if r.side(side) != start {
                                continue;
                            }
                            let ring_rect = ring_port.first_rect(ring_metal, side)?;

                            match dir {
                                Dir::Horiz => {
                                    let rect = Rect::span_builder()
                                        .with(dir, ring_rect.span(dir).union(r.span(dir)))
                                        .with(!dir, r.span(!dir))
                                        .build();
                                    ctx.draw_rect(port_metal, rect);
                                    let viap = ViaParams::builder()
                                        .layers(ring_metal, port_metal)
                                        .geometry(ring_rect, rect)
                                        .expand(substrate::layout::elements::via::ViaExpansion::LongerDirection)
                                        .build();
                                    let via = ctx.instantiate::<Via>(&viap)?;
                                    ctx.draw(via)?;
                                }
                                Dir::Vert => {
                                    let (offset, ring_rect) =
                                        if ["vpwr", "bl_dummy"].contains(&port_name) {
                                            (
                                                400,
                                                Rect::from_spans(
                                                    ring_rect.span(!dir),
                                                    Span::new(
                                                        ring_rect.side(side),
                                                        ring_rect.center().coord(dir),
                                                    ),
                                                ),
                                            )
                                        } else {
                                            (
                                                1_000,
                                                Rect::from_spans(
                                                    ring_rect.span(!dir),
                                                    Span::new(
                                                        ring_rect.center().coord(dir),
                                                        ring_rect.side(!side),
                                                    ),
                                                ),
                                            )
                                        };
                                    let overlap = 300;
                                    let rect1 = Rect::span_builder()
                                        .with(
                                            dir,
                                            r.span(dir).union(Span::new(
                                                top_limit - offset,
                                                top_limit - offset - overlap,
                                            )),
                                        )
                                        .with(!dir, r.span(!dir))
                                        .build();
                                    ctx.draw_rect(v_metal, rect1);
                                    let rect2 = Rect::span_builder()
                                        .with(
                                            dir,
                                            ring_rect.span(dir).union(Span::new(
                                                top_limit,
                                                top_limit - offset - overlap,
                                            )),
                                        )
                                        .with(!dir, r.span(!dir))
                                        .build();
                                    ctx.draw_rect(m3, rect2);
                                    let viap = ViaParams::builder()
                                        .layers(port_metal, ring_metal)
                                        .geometry(rect1, rect2)
                                        .build();
                                    let via = ctx.instantiate::<Via>(&viap)?;
                                    ctx.draw(via)?;
                                    let viap = ViaParams::builder()
                                        .layers(ring_metal, m3)
                                        .geometry(rect1, rect2)
                                        .build();
                                    let via = ctx.instantiate::<Via>(&viap)?;
                                    ctx.draw(via)?;
                                    let viap = ViaParams::builder()
                                        .layers(ring_metal, m3)
                                        .geometry(ring_rect, rect2)
                                        .build();
                                    let via = ctx.instantiate::<Via>(&viap)?;
                                    ctx.draw(via)?;
                                }
                            }
                        }
                    }
                }
            }

            let vss = array.port("ring_vss")?.into_cell_port().named("vss");
            let vdd = array.port("ring_vdd")?.into_cell_port().named("vdd");
            ctx.add_ports([vss, vdd]);
            ctx.add_ports(
                array
                    .ports()
                    .filter(|port| ["bl", "br", "wl"].contains(&port.name().as_ref())),
            );

            ctx.draw(array)?;
            Ok(())
        }
    }

    #[test]
    fn test_sp_cell_array() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array");
        let params = SpCellArrayParams {
            rows: 32,
            cols: 32,
            mux_ratio: 4,
        };
        ctx.write_layout::<SpCellArray>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");

        ctx.write_schematic_to_file::<SpCellArray>(&params, out_spice(&work_dir, "schematic"))
            .expect("failed to write schematic");
    }

    #[test]
    fn test_sp_cell_array_with_guard_ring() -> substrate::error::Result<()> {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array_with_guard_ring");

        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let params = SpCellArrayWithGuardRingParams {
            inner: SpCellArrayParams {
                rows: 32,
                cols: 32,
                mux_ratio: 4,
            },
            h_width: 1_360,
            v_width: 1_360,
        };
        ctx.write_layout::<SpCellArrayWithGuardRing>(&params, out_gds(&work_dir, "layout"))?;

        #[cfg(feature = "calibre")]
        {
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<SpCellArrayWithGuardRing>(&params, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
        Ok(())
    }

    #[test]
    fn test_sp_cell_array_tiles() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array_tiles");
        let tap_ratio = TapRatio {
            mux_ratio: 4,
            hstrap_ratio: 8,
        };
        ctx.write_layout::<SpCellArrayCornerUl>(&NoParams, out_gds(&work_dir, "corner_ul"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCornerUr>(&NoParams, out_gds(&work_dir, "corner_ur"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCornerLr>(&NoParams, out_gds(&work_dir, "corner_lr"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCornerLl>(&NoParams, out_gds(&work_dir, "corner_ll"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayBottom>(&tap_ratio, out_gds(&work_dir, "bottom"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCenter>(&tap_ratio, out_gds(&work_dir, "center"))
            .expect("failed to write layout");
    }

    #[cfg(feature = "calibre")]
    #[test]
    fn test_dff_lvs_pex() {}
}
