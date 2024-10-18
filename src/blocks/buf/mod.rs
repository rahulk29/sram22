use substrate::component::Component;

use super::gate::PrimitiveGateParams;

pub mod layout;
pub mod schematic;

pub struct DiffBuf {
    params: PrimitiveGateParams,
}

impl Component for DiffBuf {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("diff_buf")
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

    use super::layout::DiffBufCent;
    use super::*;

    const PARAMS: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 1_000,
        pwidth: 2_000,
    };

    #[test]
    fn test_diff_buf() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_diff_buf");
        ctx.write_layout::<DiffBuf>(&PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<DiffBuf>(&PARAMS, out_spice(work_dir, "schematic"))
            .expect("failed to write schematic");
    }

    #[test]
    fn test_diff_buf_cent() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_diff_buf_cent");
        ctx.write_layout::<DiffBufCent>(&PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
