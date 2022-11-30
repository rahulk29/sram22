use std::path::PathBuf;

use crate::Result;
use anyhow::Context;
use liberate_mx::{generate_lib, LibParams};

#[inline]
pub fn generate_sram_lib(params: &LibParams) -> Result<PathBuf> {
    let data = generate_lib(params).with_context(|| "Error generating Liberty file")?;

    Ok(data.lib_file)
}
