use super::ColInv;

impl ColInv {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}
