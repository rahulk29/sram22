use std::path::PathBuf;

use codegen::hard_macro;

use substrate::component::{Component, View};
use substrate::data::SubstrateCtx;

use crate::tech::{external_gds_path, external_spice_path};

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
    spice_subckt_name = "sky130_fd_bd_sram__sram_sp_cell_opt1"
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
    name = "sramgen_sp_sense_amp",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sramgen_sp_sense_amp"
)]
pub struct SenseAmp;

#[hard_macro(
    name = "sramgen_sp_sense_amp_offset",
    pdk = "sky130-open",
    path_fn = "path",
)]
pub struct SenseAmpWithOffset;

#[hard_macro(
    name = "sramgen_sp_sense_amp_cent",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sramgen_sp_sense_amp_cent"
)]
pub struct SenseAmpCent;

#[hard_macro(
    name = "openram_dff_col",
    pdk = "sky130-open",
    path_fn = "path",
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
    path_fn = "path",
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
    name = "sram_sp_hstrap",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_hstrap"
)]
pub struct SpHstrap;

#[hard_macro(
    name = "sram_sp_rowend",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend"
)]
pub struct SpRowend;

#[hard_macro(
    name = "sram_sp_rowend_hstrap",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend"
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
    name = "sram_sp_horiz_wlstrap_p",
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
