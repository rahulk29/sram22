use std::path::PathBuf;

use arcstr::ArcStr;
use codegen::hard_macro;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, View};
use substrate::data::SubstrateCtx;

use crate::tech::external_gds_path;

mod layout;
mod schematic;

fn path(_ctx: &SubstrateCtx, name: &str, view: View) -> Option<PathBuf> {
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
    use substrate::component::NoParams;

    use crate::paths::out_gds;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use crate::v2::bitcell_array::layout::*;

    use super::*;

    #[test]
    fn test_sp_cell_array() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array");
        ctx.write_layout::<SpCellArray>(
            &SpCellArrayParams {
                rows: 32,
                cols: 32,
                mux_ratio: 4,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
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
}
