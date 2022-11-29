use pdkprims::config::Int;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct Size {
    pub nmos_width: Int,
    pub pmos_width: Int,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GateParams {
    pub name: String,
    pub size: Size,
    pub length: Int,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AndParams {
    pub name: String,
    pub nand: GateParams,
    pub inv: GateParams,
}
