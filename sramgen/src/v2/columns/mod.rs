//! Column peripheral circuitry.

use substrate::component::Component;

use super::buf::DiffBufParams;
use super::precharge::PrechargeParams;
use super::rmux::ReadMuxParams;
use super::wmux::WriteMuxSizing;
use serde::Serialize;

pub mod layout;
pub mod routing;

#[derive(Debug, Clone, Serialize)]
pub struct ColParams {
    pub pc: PrechargeParams,
    pub rmux: ReadMuxParams,
    pub wmux: WriteMuxSizing,
    pub buf: DiffBufParams,
    pub cols: usize,
    pub mask_granularity: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColCentParams {
    col: ColParams,
    end: bool,
}

pub struct ColPeripherals {
    params: ColParams,
}

impl Component for ColPeripherals {
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
        arcstr::literal!("col_peripherals")
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

    use crate::paths::out_gds;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::layout::{Column, ColumnCent};
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

    const COL_PARAMS: ColParams = ColParams {
        pc: PRECHARGE_PARAMS,
        rmux: READ_MUX_PARAMS,
        wmux: WRITE_MUX_SIZING,
        buf: DIFF_BUF_PARAMS,
        cols: 128,
        mask_granularity: 8,
    };

    #[test]
    fn test_col_peripherals() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_col_peripherals");
        ctx.write_layout::<ColPeripherals>(&COL_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");

        #[cfg(feature = "calibre")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ColPeripherals>(&COL_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(output.summary, substrate::drc::DrcSummary::Pass));
        }
    }

    #[test]
    fn test_column_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_4");
        ctx.write_layout::<Column>(&COL_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_column_cent_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_cent_4");
        ctx.write_layout::<ColumnCent>(
            &ColCentParams {
                col: COL_PARAMS,
                end: false,
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
                col: COL_PARAMS,
                end: true,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }
}
