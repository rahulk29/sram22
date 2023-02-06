use std::path::PathBuf;

use arcstr::ArcStr;
use codegen::hard_macro;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, View, NoParams};
use substrate::data::SubstrateCtx;
use substrate::layout::placement::grid::ArrayTiler;

use crate::tech::external_gds_path;
fn path(_ctx: &SubstrateCtx, name: &str, view: View) -> Option<PathBuf> {
    match view {
        View::Layout => Some(external_gds_path().join(format!("{name}.gds"))),
        _ => None,
    }
}

#[hard_macro(
    name = "sramgen_control_logic_replica_v1",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sramgen_control_logic_replica_v1",
    spice_subckt_name = "sramgen_control_logic_replica_v1"
)]
pub struct ControlLogicReplicaV1;

#[hard_macro(
    name = "openram_dff",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff",
    spice_subckt_name = "sky130_fd_bd_sram__openram_dff"
)]
pub struct Dff;

pub struct DffArray {
    n: usize,
}

impl Component for DffArray {
    type Params = usize;
    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self { n: *params })
    }
    fn name(&self) -> ArcStr {
        arcstr::format!("dff_array_{}", self.n)
    }
    fn layout(&self, ctx: &mut substrate::layout::context::LayoutCtx) -> substrate::error::Result<()> {
        let mut tiler = ArrayTiler::new();
        let dff = ctx.instantiate::<Dff>(&NoParams)?;
        tiler.push_num(dff, self.n);
        ctx.draw(tiler)?;
        Ok(())
    }
}
