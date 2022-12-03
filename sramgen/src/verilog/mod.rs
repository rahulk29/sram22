use std::path::Path;

use crate::config::sram::SramParams;
use crate::{Result, TEMPLATES};

use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Sram1RwParams {
    pub module_name: String,
    pub num_words: usize,
    pub data_width: usize,
    pub addr_width: usize,
    pub wmask_width: usize,
}

pub fn generate_1rw_verilog(params: &SramParams) -> Result<String> {
    assert_eq!(params.num_words, 1 << params.addr_width);
    let template = if params.wmask_width > 1 {
        "sram_1rw_wmask.v"
    } else {
        "sram_1rw.v"
    };

    let template_params = Sram1RwParams {
        module_name: params.name.clone(),
        num_words: params.num_words,
        data_width: params.data_width,
        addr_width: params.addr_width,
        wmask_width: params.wmask_width,
    };

    Ok(TEMPLATES.render(template, &Context::from_serialize(template_params)?)?)
}

pub fn save_1rw_verilog(path: impl AsRef<Path>, params: &SramParams) -> Result<()> {
    let verilog = generate_1rw_verilog(params)?;

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, verilog)?;

    Ok(())
}
