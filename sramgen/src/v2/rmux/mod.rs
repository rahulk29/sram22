use serde::Serialize;
use substrate::component::Component;

mod layout;
mod schematic;

pub struct ReadMux {
    params: ReadMuxParams,
}

/// ReadMux taps.
pub struct ReadMuxCent {
    params: ReadMuxParams,
}

/// ReadMux end cap.
pub struct ReadMuxEnd {
    params: ReadMuxParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadMuxParams {
    pub length: i64,
    pub width: i64,
    pub mux_ratio: usize,
    pub idx: usize,
}

impl Component for ReadMux {
    type Params = ReadMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("read_mux")
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

impl Component for ReadMuxCent {
    type Params = ReadMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("read_mux_cent")
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

impl Component for ReadMuxEnd {
    type Params = ReadMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("read_mux_end")
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

    const READ_MUX_PARAMS: ReadMuxParams = ReadMuxParams {
        length: 150,
        width: 2_000,
        mux_ratio: 4,
        idx: 2,
    };

    #[test]
    fn test_read_mux() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_read_mux");
        ctx.write_layout::<ReadMux>(&READ_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_read_mux_cent() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_read_mux_cent");
        ctx.write_layout::<ReadMuxCent>(&READ_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_read_mux_end() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_read_mux_end");
        ctx.write_layout::<ReadMuxEnd>(&READ_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
