use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::blocks::sram::ControlMode;

#[derive(Debug, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub struct SramConfig {
    pub num_words: i32,
    pub data_width: i32,
    pub mux_ratio: i32,
    pub write_size: i32,
    pub control: ControlMode,
    #[cfg(feature = "commercial")]
    pub pex_level: Option<calibre::pex::PexLevel>,
}

pub fn parse_sram_config(path: impl AsRef<Path>) -> Result<SramConfig> {
    let contents = fs::read_to_string(path)?;
    let data = toml::from_str(&contents)?;
    Ok(data)
}
