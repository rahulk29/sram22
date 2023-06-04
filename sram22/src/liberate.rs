use std::path::{Path, PathBuf};

use crate::verilog::{DelayLineParams, TdcParams};
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

pub fn save_delay_line_lib(path: impl AsRef<Path>, params: &DelayLineParams) -> Result<()> {
    let verilog = generate_delay_line_lib(params)?;

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

pub fn generate_delay_line_lib(params: &DelayLineParams) -> Result<String> {
    assert!(
        params.control_width > 1,
        "Control width must be larger than 1, got {}",
        params.control_width
    );
    let template = "delay_line.fake.lib";

    Ok(TEMPLATES.render(template, &Context::from_serialize(params)?)?)
}
