use super::ReadMux;

impl ReadMux {
    pub(crate) fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}
