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

    pub fn from_u32(mut value: u32, width: usize) -> Self {
        assert!(width <= 32);
        let mut bits = Vec::with_capacity(width);
        for _ in 0..width {
            bits.push(value & 1 != 0);
            value >>= 1;
        }
        Self { bits }
    }

    pub fn from_u64(mut value: u64, width: usize) -> Self {
        assert!(width <= 64);
        let mut bits = Vec::with_capacity(width);
        for _ in 0..width {
            bits.push(value & 1 != 0);
            value >>= 1;
        }
        Self { bits }
    }

    #[inline]
    pub fn ones(width: usize) -> Self {
        Self {
            bits: vec![true; width],
        }
    }

    #[inline]
    pub fn zeros(width: usize) -> Self {
        Self {
            bits: vec![false; width],
        }
    }

    #[inline]
    pub fn from_vec(bits: Vec<bool>) -> Self {
        Self { bits }
    }

    #[inline]
    pub fn from_slice(bits: &[bool]) -> Self {
        Self {
            bits: bits.to_vec(),
        }
    }

    /// Assigns the i-th bit to the given value.
    #[inline]
    pub fn assign_bit(&mut self, i: usize, value: bool) {
        self.bits[i] = value;
    }

    /// Clears the i-th bit (ie. sets it to 0).
    #[inline]
    pub fn clear_bit(&mut self, i: usize) {
        self.bits[i] = false;
    }

    /// Sets the i-th bit (ie. sets it to 1).
    #[inline]
    pub fn set_bit(&mut self, i: usize) {
        self.bits[i] = true;
    }
}
