use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BitSignal {
    bits: Vec<bool>,
}

impl BitSignal {
    #[inline]
    pub fn width(&self) -> usize {
        self.bits.len()
    }

    pub fn bit(&self, i: usize) -> bool {
        self.bits[i]
    }

    pub fn bits(&self) -> impl Iterator<Item = bool> + '_ {
        self.bits.iter().copied()
    }
}
