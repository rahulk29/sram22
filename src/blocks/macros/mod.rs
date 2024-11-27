use std::path::PathBuf;

use codegen::hard_macro;

use crate::blocks::columns::ColumnDesignScript;
use crate::tech::{external_gds_path, external_spice_path};
use subgeom::bbox::BoundBox;
use subgeom::{Rect, Span};
use substrate::component::{Component, NoParams, View};
use substrate::data::SubstrateCtx;
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;

fn path(_ctx: &SubstrateCtx, name: &str, view: View) -> Option<PathBuf> {
    match view {
        View::Layout => Some(external_gds_path().join(format!("{name}.gds"))),
        View::Schematic => Some(external_spice_path().join(format!("{name}.spice"))),
        _ => None,
    }
}

fn layout_path(_ctx: &SubstrateCtx, name: &str, view: View) -> Option<PathBuf> {
    match view {
        View::Layout => Some(external_gds_path().join(format!("{name}.gds"))),
        _ => None,
    }
}

#[hard_macro(
    name = "sramgen_svt_inv_2",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sramgen_svt_inv_2",
    spice_subckt_name = "sramgen_svt_inv_2"
)]
pub struct SvtInv2;

#[hard_macro(
    name = "sramgen_svt_inv_4",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sramgen_svt_inv_4",
    spice_subckt_name = "sramgen_svt_inv_4"
)]
pub struct SvtInv4;

#[hard_macro(
    name = "sram_sp_cell",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cell_opt1",
    spice_subckt_name = "sram_sp_cell"
)]
pub struct SpCell;

#[hard_macro(
    name = "sram_sp_cell_replica",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_cell_opt1_replica",
    spice_subckt_name = "sram_sp_cell_replica"
)]
pub struct SpCellReplica;

#[hard_macro(
    name = "sram_sp_colend",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend"
)]
pub struct SpColend;

#[hard_macro(
    name = "sram_sp_hstrap",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_hstrap"
)]
pub struct SpHstrap;

#[hard_macro(
    name = "sramgen_sp_sense_amp",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sramgen_sp_sense_amp"
)]
pub struct SenseAmp;

#[hard_macro(
    name = "sramgen_sp_sense_amp_offset",
    pdk = "sky130-open",
    path_fn = "path"
)]
pub struct SenseAmpWithOffset;

pub struct SenseAmpCent;

impl Component for SenseAmpCent {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sense_amp_cent")
    }

    fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let outline = layers.get(Selector::Name("outline"))?;
        let tap = layers.get(Selector::Name("tap"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        let hspan = Span::new(0, pc.tap_width);
        let bounds = Rect::from_spans(hspan, sa.brect().vspan());

        ctx.draw_rect(
            nwell,
            Rect::from_spans(hspan, sa.layer_bbox(nwell).into_rect().vspan()),
        );

        let nspan = sa.layer_bbox(nsdm).into_rect().vspan();
        let pspan = sa.layer_bbox(psdm).into_rect().vspan();

        for (span, vdd) in [(pspan, true), (nspan, false)] {
            let r = Rect::from_spans(hspan, span).shrink(200);
            let viap = ViaParams::builder().layers(tap, m0).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
            let sdm_rect = via.layer_bbox(tap).into_rect().expand(130);
            ctx.draw_rect(if vdd { nsdm } else { psdm }, sdm_rect);

            let pspan = sa
                .port(if vdd { "vdd" } else { "vss" })?
                .largest_rect(m2)?
                .vspan();
            let power_stripe = Rect::from_spans(hspan, pspan);

            let viap = ViaParams::builder().layers(m0, m1).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;

            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(via.layer_bbox(m1).into_rect(), power_stripe)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            ctx.draw_rect(m2, power_stripe);

            let name = if vdd {
                arcstr::literal!("vdd")
            } else {
                arcstr::literal!("vss")
            };
            ctx.merge_port(CellPort::with_shape(name, m2, power_stripe));
        }
        ctx.draw_rect(outline, bounds);

        Ok(())
    }
}

#[hard_macro(
    name = "openram_dff",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff"
)]
pub struct Dff;

#[hard_macro(
    name = "openram_dff_col",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff_col"
)]
pub struct DffCol;

#[hard_macro(
    name = "openram_dff_col_cent",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff_col_cent"
)]
pub struct DffColCent;

#[hard_macro(
    name = "openram_dff_col_extend",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff_col_extend"
)]
pub struct DffColExtend;

#[hard_macro(
    name = "sram_sp_colend_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend_cent"
)]
pub struct SpColendCent;

#[hard_macro(
    name = "sram_sp_colend_p_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend_p_cent"
)]
pub struct SpColendPCent;

#[hard_macro(
    name = "sram_sp_corner",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_corner"
)]
pub struct SpCorner;

#[hard_macro(
    name = "sram_sp_rowend",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend"
)]
pub struct SpRowend;

#[hard_macro(
    name = "sram_sp_rowend_hstrap2",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend_hstrap"
)]
pub struct SpRowendHstrap;

#[hard_macro(
    name = "sram_sp_rowend_replica",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_rowend_replica"
)]
pub struct SpRowendReplica;

#[hard_macro(
    name = "sram_sp_wlstrap",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrap"
)]
pub struct SpWlstrap;

#[hard_macro(
    name = "sram_sp_wlstrap_p",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrap_p"
)]
pub struct SpWlstrapP;

#[hard_macro(
    name = "sram_sp_horiz_wlstrap_p2",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_horiz_wlstrap_p"
)]
pub struct SpHorizWlstrapP;

#[hard_macro(
    name = "sram_sp_cell_opt1a",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cell_opt1a",
    spice_subckt_name = "sky130_fd_bd_sram__sram_sp_cell_opt1a"
)]
pub struct SpCellOpt1a;

#[hard_macro(
    name = "sram_sp_cell_opt1a_replica",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_cell_opt1a_replica"
)]
pub struct SpCellOpt1aReplica;

#[hard_macro(
    name = "sram_sp_colenda",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda"
)]
pub struct SpColenda;

#[hard_macro(
    name = "sram_sp_colenda_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda_cent"
)]
pub struct SpColendaCent;

#[hard_macro(
    name = "sram_sp_colenda_p_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda_p_cent"
)]
pub struct SpColendaPCent;

#[hard_macro(
    name = "sram_sp_cornera",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cornera"
)]
pub struct SpCornera;

#[hard_macro(
    name = "sram_sp_rowenda",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowenda"
)]
pub struct SpRowenda;

#[hard_macro(
    name = "sram_sp_rowenda_replica",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_rowenda_replica"
)]
pub struct SpRowendaReplica;

#[hard_macro(
    name = "sram_sp_wlstrapa",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrapa"
)]
pub struct SpWlstrapa;

#[hard_macro(
    name = "sram_sp_wlstrapa_p",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrapa_p"
)]
pub struct SpWlstrapaP;

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_sense_amp_clk_cap() {
        use std::collections::HashMap;

        use substrate::component::NoParams;
        use substrate::schematic::netlist::NetlistPurpose;

        use crate::paths::{out_gds, out_spice};
        use crate::setup_ctx;
        use crate::tests::test_work_dir;

        use super::*;
        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sense_amp_clk_cap");

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<SenseAmp>(
            &NoParams,
            &pex_path,
            NetlistPurpose::Pex,
        )
        .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<SenseAmp>(&NoParams, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("sramgen_sp_sense_amp"),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("sramgen_sp_sense_amp_wrapper"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let sim_work_dir = work_dir.join("sim");
        let cap = ctx
            .write_simulation::<AcImpedanceTestbench<SenseAmp>>(
                &AcImpedanceTbParams {
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    vdd: 1.8,
                    dut: NoParams,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vss,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("VDD"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("VSS"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("clk"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("inn"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("inp"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("outp"), vec![AcImpedanceTbNode::Floating]),
                        (arcstr::literal!("outn"), vec![AcImpedanceTbNode::Floating]),
                    ]),
                },
                &sim_work_dir,
            )
            .expect("failed to write simulation");
        println!("Cclk = {}", cap.max_freq_cap());
    }
}
