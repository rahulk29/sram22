use std::path::Path;

use crate::verilog::{
    generate_delay_line_verilog, generate_tdc_verilog, DelayLineParams, TdcParams,
};
use crate::{Result, TEMPLATES};

use serde::{Deserialize, Serialize};
use tera::Context;

use super::SramParams;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Sram1RwParams {
    pub module_name: String,
    pub num_words: usize,
    pub data_width: usize,
    pub addr_width: usize,
    pub wmask_width: usize,
}

pub fn generate_1rw_verilog(name: impl Into<String>, params: &SramParams) -> Result<String> {
    assert_eq!(params.num_words, 1 << params.addr_width());

    let template_params = Sram1RwParams {
        module_name: name.into(),
        num_words: params.num_words(),
        data_width: params.data_width(),
        addr_width: params.addr_width(),
        wmask_width: params.wmask_width(),
    };

    Ok(TEMPLATES.render(
        "sram_1rw_wmask.v",
        &Context::from_serialize(template_params)?,
    )?)
}

pub fn save_1rw_verilog(
    path: impl AsRef<Path>,
    name: impl Into<String>,
    params: &SramParams,
) -> Result<()> {
    let verilog = generate_1rw_verilog(name, params)?;

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, verilog)?;

    Ok(())
}

pub fn save_tdc_verilog(path: impl AsRef<Path>, params: &TdcParams) -> Result<()> {
    let verilog = generate_tdc_verilog(params)?;

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, verilog)?;

    Ok(())
}

pub fn save_delay_line_verilog(path: impl AsRef<Path>, params: &DelayLineParams) -> Result<()> {
    let verilog = generate_delay_line_verilog(params)?;

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, verilog)?;

    Ok(())
}
