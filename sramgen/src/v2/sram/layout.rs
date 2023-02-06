use substrate::error::Result;
use substrate::layout::context::LayoutCtx;

use crate::v2::bitcell_array::{SpCellArray, SpCellArrayParams};

use super::Sram;

impl Sram {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let bitcells = ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows,
            cols: self.params.cols,
            mux_ratio: self.params.mux_ratio,
        })?;
        ctx.draw(bitcells)?;
        Ok(())
    }
}
