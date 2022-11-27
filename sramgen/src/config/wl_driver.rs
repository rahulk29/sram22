use pdkprims::config::Int;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WordlineDriverParams {
    pub name: String,
    pub length: Int,
    pub nand_size: Size,
    pub inv_size: Size,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WordlineDriverArrayParams {
    pub name: String,
    pub width: i64,
    pub instance_params: WordlineDriverParams,
}
