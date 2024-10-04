use serde::Serialize;
use substrate::component::Component;

pub mod layout;
pub mod schematic;

pub struct WriteDriver {
    params: WriteDriverParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteDriverParams {
    pub length: i64,
    pub pwidth_driver: i64,
    pub nwidth_driver: i64,
}

impl Component for WriteDriver {
    type Params = WriteDriverParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_driver")
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

    const WRITE_DRIVER_PARAMS: WriteDriverParams = WriteDriverParams {
        length: 150,
        pwidth_driver: 2_000,
        nwidth_driver: 2_000,
    };

    #[test]
    fn test_write_driver() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_driver");
        ctx.write_schematic_to_file::<WriteDriver>(
            &WRITE_DRIVER_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        ctx.write_layout::<WriteDriver>(&WRITE_DRIVER_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
    }
}
