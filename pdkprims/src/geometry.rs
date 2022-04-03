use serde::{Deserialize, Serialize};

/// A direction: horizontal or vertical.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoarseDirection {
    Horizontal,
    Vertical,
}

impl Default for CoarseDirection {
    fn default() -> Self {
        Self::Vertical
    }
}
