use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SenseAmpArrayParams {
    pub name: String,
    pub width: i64,
}
