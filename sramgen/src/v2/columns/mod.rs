//! Column peripheral circuitry.

use substrate::component::Component;
use substrate::layout::context::LayoutCtx;
use substrate::schematic::context::SchematicCtx;

use super::buf::DiffBufParams;
use super::precharge::PrechargeParams;
use super::rmux::ReadMuxParams;
use super::wmux::WriteMuxSizing;
use serde::Serialize;

pub mod layout;
pub mod schematic;

#[derive(Debug, Clone, Serialize)]
pub struct ColParams {
    pub pc: PrechargeParams,
    pub rmux: ReadMuxParams,
    pub wmux: WriteMuxSizing,
    pub buf: DiffBufParams,
    pub cols: usize,
    pub include_wmask: bool,
    pub wmask_granularity: usize,
}

impl ColParams {
    fn mux_ratio(&self) -> usize {
        self.rmux.mux_ratio
    }

    fn word_length(&self) -> usize {
        self.cols / self.mux_ratio()
    }

    fn wmask_bits(&self) -> usize {
        self.word_length() / self.wmask_granularity
    }
}

pub struct ColPeripherals {
    params: ColParams,
}

pub struct Column {
    params: ColParams,
}

impl Component for ColPeripherals {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        if params.rmux.mux_ratio != params.wmux.mux_ratio {
            return Err(substrate::error::ErrorSource::Component(
                substrate::component::error::Error::InvalidParams,
            )
            .into());
        }
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("col_peripherals")
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for Column {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column")
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::layout::{ColCentParams, ColumnCent};
    use super::*;

    const WRITE_MUX_SIZING: WriteMuxSizing = WriteMuxSizing {
        length: 150,
        mux_width: 2_000,
        mux_ratio: 4,
    };
    const READ_MUX_PARAMS: ReadMuxParams = ReadMuxParams {
        length: 150,
        width: 2_000,
        mux_ratio: 4,
        idx: 2,
    };
    const PRECHARGE_PARAMS: PrechargeParams = PrechargeParams {
        length: 150,
        pull_up_width: 1_600,
        equalizer_width: 1_000,
    };
    const DIFF_BUF_PARAMS: DiffBufParams = DiffBufParams {
        width: 4_800,
        nw: 1_200,
        pw: 2_000,
        lch: 150,
    };

    const COL_WMASK_PARAMS: ColParams = ColParams {
        pc: PRECHARGE_PARAMS,
        rmux: READ_MUX_PARAMS,
        wmux: WRITE_MUX_SIZING,
        buf: DIFF_BUF_PARAMS,
        cols: 128,
        include_wmask: true,
        wmask_granularity: 8,
    };

    const COL_PARAMS: ColParams = ColParams {
        pc: PRECHARGE_PARAMS,
        rmux: READ_MUX_PARAMS,
        wmux: WRITE_MUX_SIZING,
        buf: DIFF_BUF_PARAMS,
        cols: 128,
        include_wmask: false,
        wmask_granularity: 8,
    };

    #[test]
    fn test_col_peripherals() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_col_peripherals");
        ctx.write_layout::<ColPeripherals>(&COL_WMASK_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<ColPeripherals>(
            &COL_WMASK_PARAMS,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ColPeripherals>(&COL_WMASK_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
        }
    }

    #[test]
    fn test_column_wmask_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_wmask_4");
        ctx.write_layout::<Column>(&COL_WMASK_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_column_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_4");
        ctx.write_layout::<Column>(&COL_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<Column>(&COL_PARAMS, out_spice(work_dir, "schematic"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_column_cent_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_cent_4");
        ctx.write_layout::<ColumnCent>(
            &ColCentParams {
                col: COL_WMASK_PARAMS,
                end: false,
                cut_wmask: false,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }

    #[test]
    fn test_column_end_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_end_4");
        ctx.write_layout::<ColumnCent>(
            &ColCentParams {
                col: COL_WMASK_PARAMS,
                end: true,
                cut_wmask: true,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }
}
