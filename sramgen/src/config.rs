use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub struct SramConfig {
    pub num_words: i32,
    pub data_width: i32,
    pub mux_ratio: i32,
    pub write_size: i32,
    pub control: ControlMode,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum ControlMode {
    Simple,
    SimpleChipSelect,
    Replica,
}

fn parse_config(path: impl AsRef<Path>) -> Result<SramConfig> {
    let contents = fs::read_to_string(path)?;
    let data = toml::from_str(&contents)?;
    Ok(data)
}
