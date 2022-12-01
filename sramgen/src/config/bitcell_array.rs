use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BitcellArrayDummyParams {
    pub top: usize,
    pub bottom: usize,
    pub left: usize,
    pub right: usize,
}

impl BitcellArrayDummyParams {
    pub fn equal(all: usize) -> Self {
        Self {
            top: all,
            bottom: all,
            left: all,
            right: all,
        }
    }
    pub fn symmetric(rows: usize, cols: usize) -> Self {
        Self {
            top: rows,
            bottom: rows,
            left: cols,
            right: cols,
        }
    }

    pub fn enumerate(top: usize, bottom: usize, left: usize, right: usize) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BitcellArrayParams {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub replica_cols: usize,
    pub dummy_params: BitcellArrayDummyParams,
}
