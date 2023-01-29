//! Column peripheral circuitry.

use substrate::component::Component;

use super::precharge::PrechargeParams;
use super::rmux::ReadMuxParams;
use super::wmux::WriteMuxSizing;
use serde::Serialize;

pub mod layout;

#[derive(Debug, Clone, Serialize)]
pub struct ColParams {
    pc: PrechargeParams,
    rmux: ReadMuxParams,
    wmux: WriteMuxSizing,
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

    const COL_PARAMS: ColParams = ColParams {
        pc: PRECHARGE_PARAMS,
        rmux: READ_MUX_PARAMS,
        wmux: WRITE_MUX_SIZING,
    };

    #[test]
    fn test_col_peripherals() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_col_peripherals");
        ctx.write_layout::<ColPeripherals>(&COL_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
