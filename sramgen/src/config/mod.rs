use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::path::Path;

pub mod sram;

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
    ReplicaV1,
}

impl Display for ControlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Simple => write!(f, "simple"),
            Self::ReplicaV1 => write!(f, "replica_v1"),
        }
    }
}

pub fn parse_config(path: impl AsRef<Path>) -> Result<SramConfig> {
    let contents = fs::read_to_string(path)?;
    let data = toml::from_str(&contents)?;
    Ok(data)
}
