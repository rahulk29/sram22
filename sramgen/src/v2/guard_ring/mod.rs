use serde::{Deserialize, Serialize};
use substrate::component::Component;
use substrate::layout::geom::ring::Ring;
use substrate::layout::geom::Rect;
use substrate::layout::layers::LayerKey;

pub struct GuardRing {
    params: GuardRingParams,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuardRingParams {
    enclosure: Rect,
    h_metal: LayerKey,
    v_metal: LayerKey,
    h_width: i64,
    v_width: i64,
}

impl Component for GuardRing {
    type Params = GuardRingParams;
    fn new(
        params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("guard_ring")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let vss_ring = Ring::builder();
        Ok(())
    }
}
