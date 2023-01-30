use serde::Serialize;
use substrate::component::Component;

pub mod layout;
pub mod schematic;

pub struct ColInv {
    params: ColInvParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColInvParams {
    pub length: i64,
    pub nwidth: i64,
    pub pwidth: i64,
}

impl Component for ColInv {
    type Params = ColInvParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("precharge")
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

    #[test]
    fn test_col_inv() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_col_inv");

        let params = ColInvParams {
            length: 150,
            nwidth: 1_400,
            pwidth: 2_600,
        };
        ctx.write_layout::<ColInv>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<ColInv>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");
    }
}
