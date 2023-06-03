use std::path::{Path, PathBuf};

use crate::verilog::TdcParams;
use crate::{Result, TEMPLATES};
use anyhow::Context as AnyhowContext;
use liberate_mx::{generate_lib, LibParams};
use tera::Context;

#[inline]
pub fn generate_sram_lib(params: &LibParams) -> Result<PathBuf> {
    let data = generate_lib(params).with_context(|| "Error generating Liberty file")?;

    Ok(data.lib_file)
}

pub fn save_tdc_lib(path: impl AsRef<Path>, params: &TdcParams) -> Result<()> {
    let verilog = generate_tdc_lib(params)?;

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, verilog)?;

    Ok(())
}

pub fn generate_tdc_lib(params: &TdcParams) -> Result<String> {
    assert!(
        params.data_width > 1,
        "Output width must be larger than 1, got {}",
        params.data_width
    );
    let template = "tdc.fake.lib";

    Ok(TEMPLATES.render(template, &Context::from_serialize(params)?)?)
}
