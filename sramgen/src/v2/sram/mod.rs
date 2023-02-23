use serde::{Deserialize, Serialize};
use substrate::component::Component;

pub mod layout;
pub mod schematic;

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
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    const PARAMS_1: SramParams = SramParams {
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
    const PARAMS_2: SramParams = SramParams {
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
        ctx.write_schematic_to_file::<Sram>(&PARAMS_1, out_spice(work_dir, "schematic"))
            .expect("failed to write schematic");
    }

    #[test]
    #[ignore = "slow"]
    fn test_sram_2() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sram_2");
        ctx.write_layout::<Sram>(&PARAMS_2, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
