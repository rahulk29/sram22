use serde::{Deserialize, Serialize};
use substrate::component::Component;

pub mod layout;
pub mod schematic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffBufParams {
    pub width: i64,
    pub nw: i64,
    pub pw: i64,
    pub lch: i64,
}

pub struct DiffBuf {
    params: DiffBufParams,
}

impl Component for DiffBuf {
    type Params = DiffBufParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
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

    const PARAMS: DiffBufParams = DiffBufParams {
        lch: 150,
        nw: 1_000,
        pw: 2_000,
        width: 4_800,
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
