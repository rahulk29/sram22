use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::{Dir, ExpandMode, Rect, Span};
use substrate::component::Component;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::routing::auto::straps::PlacedStraps;
use substrate::layout::straps::SingleSupplyNet;

use super::guard_ring::{GuardRing, GuardRingParams, GuardRingWrapper, SupplyRings, WrapperParams};

pub mod layout;
pub mod schematic;
pub mod testbench;

pub struct SramInner {
    params: SramParams,
}

pub struct Sram {
    params: SramParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SramParams {
    pub wmask_width: usize,

    // Schematic
    pub row_bits: usize,
    pub col_bits: usize,
    pub col_select_bits: usize,

    // Layout
    pub rows: usize,
    pub cols: usize,
    pub mux_ratio: usize,

    // Verilog
    pub num_words: usize,
    pub data_width: usize,
    pub addr_width: usize,

    pub control: ControlMode,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum ControlMode {
    Simple,
    ReplicaV1,
}

impl Component for SramInner {
    type Params = SramParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sramgen_sram_inner")
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

impl Component for Sram {
    type Params = SramParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sramgen_sram")
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let mut inner = ctx.instantiate::<SramInner>(&self.params)?;
        ctx.bubble_all_ports(&mut inner);
        ctx.add_instance(inner);
        Ok(())
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let sram = ctx.instantiate::<SramInner>(&self.params)?;
        let brect = sram.brect();
        ctx.add_ports(sram.ports());
        ctx.draw_ref(&sram)?;

        let m1 = ctx.layers().get(Selector::Metal(1))?;
        let m2 = ctx.layers().get(Selector::Metal(2))?;
        let m3 = ctx.layers().get(Selector::Metal(3))?;
        let params = GuardRingParams {
            enclosure: brect.expand(1_000),
            h_metal: m2,
            v_metal: m1,
            h_width: 1_360,
            v_width: 1_360,
        };
        let ring = ctx.instantiate::<GuardRing>(&params)?;
        let rings = ring.cell().get_metadata::<SupplyRings>();
        let straps = sram.cell().get_metadata::<PlacedStraps>();

        for (layer, dir) in [(m2, Dir::Horiz), (m3, Dir::Vert)] {
            for strap in straps.on_layer(layer) {
                let ring = match strap.net {
                    SingleSupplyNet::Vss => rings.vss,
                    SingleSupplyNet::Vdd => rings.vdd,
                };
                assert_ne!(strap.rect.area(), 0);
                let lower = if strap.lower_boundary {
                    ring.outer().span(dir).start()
                } else {
                    strap.rect.span(dir).stop()
                };
                let upper = if strap.upper_boundary {
                    ring.outer().span(dir).stop()
                } else {
                    strap.rect.span(dir).start()
                };

                let r = Rect::span_builder()
                    .with(dir, Span::new(lower, upper))
                    .with(!dir, strap.rect.span(!dir))
                    .build();

                let below = if layer == m2 { m1 } else { m2 };

                if strap.upper_boundary {
                    let target = ring.dir_rects(!dir)[1];
                    let viap = ViaParams::builder()
                        .layers(below, layer)
                        .geometry(target, r)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    ctx.instantiate::<Via>(&viap)?.add_to(ctx)?;
                }
                if strap.lower_boundary {
                    let target = ring.dir_rects(!dir)[0];
                    let viap = ViaParams::builder()
                        .layers(below, layer)
                        .geometry(target, r)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    ctx.instantiate::<Via>(&viap)?.add_to(ctx)?;
                }
                ctx.draw_rect(layer, r);
            }
        }

        ctx.draw(ring)?;

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    pub(crate) const TINY_SRAM: SramParams = SramParams {
        wmask_width: 2,
        row_bits: 4,
        col_bits: 4,
        col_select_bits: 2,
        rows: 16,
        cols: 16,
        mux_ratio: 4,
        num_words: 64,
        data_width: 4,
        addr_width: 6,
        control: ControlMode::ReplicaV1,
    };

    pub(crate) const PARAMS_1: SramParams = SramParams {
        wmask_width: 4,
        row_bits: 6,
        col_bits: 7,
        col_select_bits: 2,
        rows: 64,
        cols: 128,
        mux_ratio: 4,
        num_words: 256,
        data_width: 32,
        addr_width: 8,
        control: ControlMode::ReplicaV1,
    };

    pub(crate) const PARAMS_2: SramParams = SramParams {
        wmask_width: 8,
        row_bits: 9,
        col_bits: 8,
        col_select_bits: 2,
        rows: 512,
        cols: 256,
        mux_ratio: 4,
        num_words: 2048,
        data_width: 64,
        addr_width: 11,
        control: ControlMode::ReplicaV1,
    };

    #[test]
    fn test_sram_1() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sram_1");
        ctx.write_layout::<Sram>(&PARAMS_1, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<SramInner>(&PARAMS_1, out_spice(work_dir, "schematic"))
            .expect("failed to write schematic");
    }

    #[test]
    #[ignore = "slow"]
    fn test_sram_2() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sram_2");
        ctx.write_layout::<SramInner>(&PARAMS_2, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
