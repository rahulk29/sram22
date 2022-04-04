use std::fmt::Display;

use layout21::raw::{BoundBox, Rect};
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

pub fn expand_box(b: &mut Rect, dist: crate::config::Int) {
    assert!(b.p0.x <= b.p1.x);
    assert!(b.p0.y <= b.p1.y);

    b.p0.x -= dist;
    b.p1.x += dist;
    b.p0.y -= dist;
    b.p1.y += dist;
}

fn rect_from_bbox(bbox: &BoundBox) -> Rect {
    Rect {
        p0: bbox.p0.clone(),
        p1: bbox.p1.clone(),
    }
}
