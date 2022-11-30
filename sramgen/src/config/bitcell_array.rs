use serde::{Deserialize, Serialize};

#[derive(Copy, Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BitcellArrayDummyParams {
    Equal(usize),
    Symmetric {
        rows: usize,
        cols: usize,
    },
    Custom {
        top: usize,
        bottom: usize,
        left: usize,
        right: usize,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BitcellArrayParams {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub replica_cols: usize,
    pub dummy_params: BitcellArrayDummyParams,
}
