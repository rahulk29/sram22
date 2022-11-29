use pdkprims::config::Int;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SenseAmpArrayParams {
    pub name: String,
    pub width: usize,
    pub spacing: Option<Int>,
}
