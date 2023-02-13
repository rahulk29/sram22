use serde::{Deserialize, Serialize};
use substrate::component::Component;

mod layout;
mod schematic;

pub struct WriteMux {
    params: WriteMuxParams,
}

/// WriteMux taps.
pub struct WriteMuxCent {
    params: WriteMuxCentParams,
}

/// WriteMux end cap.
pub struct WriteMuxEnd {
    params: WriteMuxEndParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WriteMuxSizing {
    pub length: i64,
    pub mux_width: i64,
    pub mux_ratio: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteMuxParams {
    pub sizing: WriteMuxSizing,
    pub idx: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteMuxCentParams {
    pub sizing: WriteMuxSizing,
    /// Whether to cut the data line between adjacent muxes.
    pub cut_data: bool,
    /// Whether to cut the wmask line between adjacent muxes.
    pub cut_wmask: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteMuxEndParams {
    pub sizing: WriteMuxSizing,
}

impl WriteMuxCentParams {
    pub(crate) fn for_wmux(&self) -> WriteMuxParams {
        WriteMuxParams {
            sizing: self.sizing,
            idx: 0,
        }
    }
}

impl WriteMuxEndParams {
    pub(crate) fn for_wmux_cent(&self) -> WriteMuxCentParams {
        WriteMuxCentParams {
            sizing: self.sizing,
            cut_data: true,
            cut_wmask: true,
        }
    }
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
    type Params = WriteMuxCentParams;
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
    type Params = WriteMuxEndParams;
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

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    const WRITE_MUX_SIZING: WriteMuxSizing = WriteMuxSizing {
        length: 150,
        mux_width: 2_000,
        mux_ratio: 4,
    };

    const WRITE_MUX_PARAMS: WriteMuxParams = WriteMuxParams {
        sizing: WRITE_MUX_SIZING,
        idx: 2,
    };
    const WRITE_MUX_CENT_PARAMS: WriteMuxCentParams = WriteMuxCentParams {
        sizing: WRITE_MUX_SIZING,
        cut_data: true,
        cut_wmask: false,
    };
    const WRITE_MUX_END_PARAMS: WriteMuxEndParams = WriteMuxEndParams {
        sizing: WRITE_MUX_SIZING,
    };

    #[test]
    fn test_write_mux() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux");
        ctx.write_layout::<WriteMux>(&WRITE_MUX_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<WriteMux>(
            &WRITE_MUX_PARAMS,
            out_spice(work_dir, "schematic"),
        )
        .expect("failed to write schematic");
    }

    #[test]
    fn test_write_mux_cent() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux_cent");
        ctx.write_layout::<WriteMuxCent>(&WRITE_MUX_CENT_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_write_mux_end() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux_end");
        ctx.write_layout::<WriteMuxEnd>(&WRITE_MUX_END_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
