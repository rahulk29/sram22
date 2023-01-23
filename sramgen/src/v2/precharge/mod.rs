use serde::Serialize;
use substrate::component::Component;

mod layout;
mod schematic;

pub struct Precharge {
    params: PrechargeParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrechargeParams {
    pub name: String,
    pub length: i64,
    pub pull_up_width: i64,
    pub equalizer_width: i64,
}

impl Component for Precharge {
    type Params = PrechargeParams;
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
    use crate::paths::out_gds;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    fn test_precharge() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_precharge");
        ctx.write_layout::<Precharge>(
            &PrechargeParams {
                name: "precharge".into(),
                length: 150,
                pull_up_width: 1_600,
                equalizer_width: 1_000,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }
}
