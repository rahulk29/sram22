use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub struct SramConfig {
    pub name: String,
    pub rows: i32,
    pub cols: i32,
    pub mux_ratio: i32,
    pub wmask_bits: Option<i32>,
    pub control: ControlMode,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum ControlMode {
    Simple,
    SimpleChipSelect,
    Replica,
}
