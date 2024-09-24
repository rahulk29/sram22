use arcstr::ArcStr;

use serde::{Deserialize, Serialize};
use subgeom::{Dir, Rect, Shape, Side, Span};
use substrate::component::Component;
use substrate::layout::cell::{Port, PortId};
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;

use crate::blocks::guard_ring::{GuardRingWrapper, WrapperParams};

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
        if params.rows % 4 != 0
            || params.cols % params.mux_ratio != 0
            || params.rows == 0
            || params.cols == 0
        {
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
        Ok(Self { params: *params })
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

        for (ring_port_name, side, port_names) in [
            ("ring_vss", Side::Left, vec!["vgnd", "vnb", "wl_dummy"]),
            ("ring_vdd", Side::Left, vec!["vpwr", "vpb"]),
            ("ring_vss", Side::Top, vec!["vgnd", "vnb"]),
            (
                "ring_vdd",
                Side::Top,
                vec!["vpwr", "vpb", "bl_dummy", "br_dummy"],
            ),
        ] {
            let dir = side.coord_dir();
            let (ring_metal, port_metal) = match dir {
                Dir::Horiz => (v_metal, h_metal),
                Dir::Vert => (h_metal, v_metal),
            };
            let ring_port = array.port(ring_port_name)?;
            let ring_rect = ring_port.first_rect(ring_metal, side)?;
            // Specifically for routing vertical straps to VDD via M3.
            let mut via_rects = Vec::new();
            // Specifically for merging horizontal straps on same net.
            let mut to_merge = Vec::new();

            for port_name in port_names {
                let ports = array.ports_starting_with(port_name);
                for port in ports {
                    if port.id() == &PortId::new("bl_dummy", 0)
                        || port.id() == &PortId::new("br_dummy", 0)
                    {
                        continue;
                    }
                    let start = port.first_rect(port_metal, side)?.side(side);
                    let extremes = Side::with_dir(side.edge_dir())
                        .map(|s| port.first_rect(port_metal, s).unwrap().side(s))
                        .collect::<Vec<i64>>();
                    let intermediate_vspan = Span::with_point_and_length(
                        !side.sign(),
                        ring_rect.side(!side) - side.sign().as_int() * 4_000,
                        500,
                    );
                    if side == Side::Top && port_name == "vpb" {
                        // Special case for clustered VDD ports at the edges.
                        for edge in Side::with_dir(!dir) {
                            let edge_start = port.first_rect(port_metal, edge)?.side(edge);
                            let hspan = Span::with_point_and_length(edge.sign(), edge_start, 1_400);
                            let rect = Rect::from_spans(hspan, intermediate_vspan);
                            ctx.draw_rect(port_metal, rect);
                            let viap = ViaParams::builder()
                                .layers(port_metal, ring_metal)
                                .geometry(rect, rect)
                                .expand(
                                    substrate::layout::elements::via::ViaExpansion::LongerDirection,
                                )
                                .build();
                            let via = ctx.instantiate::<Via>(&viap)?;
                            ctx.draw(via)?;
                            let viap = ViaParams::builder()
                                .layers(ring_metal, m3)
                                .geometry(rect, rect)
                                .expand(
                                    substrate::layout::elements::via::ViaExpansion::LongerDirection,
                                )
                                .build();
                            let via = ctx.instantiate::<Via>(&viap)?;
                            via_rects.push(via.layer_bbox(m3).into_rect());
                            ctx.draw(via)?;
                        }
                    }

                    for shape in port.shapes(port_metal) {
                        if let Shape::Rect(r) = shape {
                            if r.side(side) != start {
                                continue;
                            }

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
                                    to_merge.push(via.layer_bbox(port_metal).into_rect());
                                    ctx.draw(via)?;
                                }
                                Dir::Vert => match ring_port_name {
                                    "ring_vss" => {
                                        let rect = Rect::span_builder()
                                            .with(dir, ring_rect.span(dir).union(r.span(dir)))
                                            .with(!dir, r.span(!dir))
                                            .build();
                                        ctx.draw_rect(port_metal, rect);
                                    }
                                    _ => {
                                        let rect = Rect::span_builder()
                                            .with(dir, intermediate_vspan.union(r.span(dir)))
                                            .with(!dir, r.span(!dir))
                                            .build();
                                        ctx.draw_rect(port_metal, rect);

                                        if !(extremes.contains(&r.left())
                                            || extremes.contains(&r.right()))
                                        {
                                            let viap = ViaParams::builder()
                                                .layers(port_metal, ring_metal)
                                                .geometry(
                                                    rect.with_vspan(intermediate_vspan),
                                                    rect.with_vspan(intermediate_vspan),
                                                )
                                                .build();
                                            let via = ctx.instantiate::<Via>(&viap)?;
                                            ctx.draw(via)?;
                                            let viap = ViaParams::builder()
                                                .layers(ring_metal, m3)
                                                .geometry(
                                                    rect.with_vspan(intermediate_vspan),
                                                    rect.with_vspan(intermediate_vspan),
                                                )
                                                .build();
                                            let via = ctx.instantiate::<Via>(&viap)?;
                                            via_rects.push(via.layer_bbox(m3).into_rect());
                                            ctx.draw(via)?;
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
            }

            // Only applicable for VDD connections on the top of the bitcell.
            for via in via_rects {
                let rect = Rect::from_spans(
                    via.span(side.edge_dir()),
                    via.span(side.coord_dir()).union(ring_rect.vspan()),
                );
                ctx.draw_rect(m3, rect);
                let viap = ViaParams::builder()
                    .layers(ring_metal, m3)
                    .geometry(ring_rect, rect)
                    .expand(substrate::layout::elements::via::ViaExpansion::LongerDirection)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw(via)?;
            }

            // Only applicable for metal 2 horizontal connections.
            if let Some(hspan) = to_merge
                .iter()
                .map(|rect| rect.span(dir))
                .reduce(|acc, e| acc.union(e))
            {
                let merged_spans =
                    Span::merge_adjacent(to_merge.iter().map(|rect| rect.span(!dir)), |a, b| {
                        a.min_distance(b) < 400
                    });
                for span in merged_spans {
                    let curr = Rect::span_builder()
                        .with(dir, hspan)
                        .with(!dir, span)
                        .build();
                    ctx.draw_rect(port_metal, curr);
                }
            }
        }

        let vss = array.port("ring_vss")?.into_cell_port().named("vss");
        let vdd = array.port("ring_vdd")?.into_cell_port().named("vdd");
        ctx.add_ports([vss, vdd]).unwrap();
        ctx.add_ports(
            array
                .ports()
                .filter(|port| ["bl", "br", "wl"].contains(&port.name().as_ref())),
        )
        .unwrap();
        ctx.add_port(array.port("bl_dummy")?.into_cell_port().named("dummy_bl"))
            .unwrap();
        ctx.add_port(array.port("br_dummy")?.into_cell_port().named("dummy_br"))
            .unwrap();

        ctx.draw(array)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use substrate::component::NoParams;
    use substrate::schematic::netlist::NetlistPurpose;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::layout::{
        SpCellArrayBottom, SpCellArrayCenter, SpCellArrayCornerLl, SpCellArrayCornerLr,
        SpCellArrayCornerUl, SpCellArrayCornerUr, TapRatio,
    };
    use super::*;

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

        let params = SpCellArrayWithGuardRingParams {
            inner: SpCellArrayParams {
                rows: 16,
                cols: 16,
                mux_ratio: 4,
            },
            h_width: 1_360,
            v_width: 1_360,
        };
        ctx.write_layout::<SpCellArrayWithGuardRing>(&params, out_gds(&work_dir, "layout"))?;

        ctx.write_schematic_to_file::<SpCellArrayWithGuardRing>(
            &params,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<SpCellArrayWithGuardRing>(&params, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
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
            hstrap_ratio: 4,
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

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_bitline_wordline_cap() {
        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_bitline_wordline_cap");
        let params = SpCellArrayWithGuardRingParams {
            inner: SpCellArrayParams {
                rows: 128,
                cols: 8,
                mux_ratio: 4,
            },
            h_width: 1_360,
            v_width: 1_360,
        };

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<SpCellArrayWithGuardRing>(
            &params,
            &pex_path,
            NetlistPurpose::Pex,
        )
        .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<SpCellArrayWithGuardRing>(&params, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("sp_cell_array_with_guard_ring"),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("sp_cell_array_with_guard_ring"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let mut bls = vec![AcImpedanceTbNode::Vdd; params.inner.cols];
        bls[0] = AcImpedanceTbNode::Vmeas;

        let bl_ac_work_dir = work_dir.join("bl_ac_sim");
        let cap_ac = ctx
            .write_simulation::<AcImpedanceTestbench<SpCellArrayWithGuardRing>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("dummy_bl"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("dummy_br"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("bl"), bls),
                        (
                            arcstr::literal!("br"),
                            vec![AcImpedanceTbNode::Vdd; params.inner.cols],
                        ),
                        (
                            arcstr::literal!("wl"),
                            vec![AcImpedanceTbNode::Vss; params.inner.rows],
                        ),
                    ]),
                },
                &bl_ac_work_dir,
            )
            .expect("failed to write simulation");

        println!("Cbl = {}", cap_ac.max_freq_cap());

        let mut wls = vec![AcImpedanceTbNode::Vss; params.inner.rows];
        wls[0] = AcImpedanceTbNode::Vmeas;

        let wl_ac_work_dir = work_dir.join("wl_ac_sim");
        let cap_ac = ctx
            .write_simulation::<AcImpedanceTestbench<SpCellArrayWithGuardRing>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path),
                    vmeas_conn: AcImpedanceTbNode::Vss,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("dummy_bl"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("dummy_br"), vec![AcImpedanceTbNode::Vdd]),
                        (
                            arcstr::literal!("bl"),
                            vec![AcImpedanceTbNode::Vdd; params.inner.cols],
                        ),
                        (
                            arcstr::literal!("br"),
                            vec![AcImpedanceTbNode::Vdd; params.inner.cols],
                        ),
                        (arcstr::literal!("wl"), wls),
                    ]),
                },
                &wl_ac_work_dir,
            )
            .expect("failed to write simulation");

        println!("Cwl = {}", cap_ac.max_freq_cap());
    }
}
