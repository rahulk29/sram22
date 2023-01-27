use super::WriteMux;

impl WriteMux {
    pub(crate) fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}
