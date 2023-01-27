use serde::Serialize;
use substrate::component::Component;

mod layout;
mod schematic;

pub struct WriteMux {
    params: WriteMuxParams,
}

/// WriteMux taps.
pub struct WriteMuxCent {
    params: WriteMuxParams,
}

/// WriteMux end cap.
pub struct WriteMuxEnd {
    params: WriteMuxParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteMuxParams {
    pub length: i64,
    pub mux_width: i64,
    pub mux_ratio: usize,
    pub idx: usize,
}

impl Component for WriteMux {
    type Params = WriteMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_mux")
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

impl Component for WriteMuxCent {
    type Params = WriteMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_mux_cent")
    }

    fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for WriteMuxEnd {
    type Params = WriteMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_mux_end")
    }

    fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
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

    const WRITE_MUX_PARAMS: WriteMuxParams = WriteMuxParams {
        length: 150,
        mux_width: 2_000,
        mux_ratio: 4,
        idx: 2,
    };

    #[test]
    fn test_write_mux() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux");
        ctx.write_layout::<WriteMux>(&WRITE_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_write_mux_cent() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux_cent");
        ctx.write_layout::<WriteMuxCent>(&WRITE_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_write_mux_end() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux_end");
        ctx.write_layout::<WriteMuxEnd>(&WRITE_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
