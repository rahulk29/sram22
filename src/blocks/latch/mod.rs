use super::gate::PrimitiveGateParams;
use serde::{Deserialize, Serialize};
use substrate::component::Component;

pub mod layout;
pub mod schematic;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct DiffLatchParams {
    pub inv_in: PrimitiveGateParams,
    pub invq: PrimitiveGateParams,
    pub inv_out: PrimitiveGateParams,
    pub nwidth: i64,
    pub lch: i64,
}

pub struct DiffLatch {
    params: DiffLatchParams,
}

impl Component for DiffLatch {
    type Params = DiffLatchParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("diff_latch")
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

    use crate::blocks::columns::DIFF_LATCH_PARAMS;
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::layout::DiffLatchCent;
    use super::*;

    #[test]
    fn test_diff_latch() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_diff_latch");
        ctx.write_layout::<DiffLatch>(&DIFF_LATCH_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<DiffLatch>(
            &DIFF_LATCH_PARAMS,
            out_spice(work_dir, "schematic"),
        )
        .expect("failed to write schematic");
    }

    #[test]
    fn test_diff_latch_cent() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_diff_latch_cent");
        ctx.write_layout::<DiffLatchCent>(&DIFF_LATCH_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
