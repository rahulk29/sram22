use std::fmt::Display;

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

impl Display for CoarseDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Horizontal => write!(f, "horizontal"),
            Self::Vertical => write!(f, "vertical"),
        }
    }
}

impl CoarseDirection {
    pub fn short_form(&self) -> &'static str {
        match *self {
            Self::Horizontal => "h",
            Self::Vertical => "v",
        }
    }
}
