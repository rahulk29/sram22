use std::fmt::Display;

use crate::config::Int;
use layout21::raw::{BoundBox, Point, Rect};
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

pub fn box_width(b: &mut Rect) -> Int {
    b.p1.x - b.p0.x
}
pub fn box_height(b: &mut Rect) -> Int {
    b.p1.y - b.p0.y
}

pub fn expand_box(b: &mut Rect, dist: Int) {
    assert!(b.p0.x <= b.p1.x);
    assert!(b.p0.y <= b.p1.y);

    b.p0.x -= dist;
    b.p1.x += dist;
    b.p0.y -= dist;
    b.p1.y += dist;
}

pub fn expand_box_min_width(b: &mut Rect, width: Int, grid: Int) {
    assert!(width >= 0);
    assert!(b.p0.x <= b.p1.x);
    assert!(b.p0.y <= b.p1.y);
    assert!(width % grid == 0);

    let cwidth = b.p1.x - b.p0.x;
    if cwidth < width && (width - cwidth) % (2 * grid) == 0 {
        let ofsx = (width - cwidth) / 2;
        assert!(ofsx > 0);
        b.p0.x -= ofsx;
        b.p1.x += ofsx;
    }

    let cheight = b.p1.y - b.p0.y;
    if cheight < width && (width - cheight) % (2 * grid) == 0 {
        let ofsy = (width - cheight) / 2;
        assert!(ofsy > 0);
        b.p0.y -= ofsy;
        b.p1.y += ofsy;
    }

    assert!(box_width(b) >= width);
    assert!(box_height(b) >= width);
    assert!(box_width(b) >= cwidth);
    assert!(box_height(b) >= cheight);
}

pub fn rect_from_bbox(bbox: &BoundBox) -> Rect {
    Rect {
        p0: bbox.p0,
        p1: bbox.p1,
    }
}

pub fn translate(r: &Rect, p: &Point) -> Rect {
    Rect {
        p0: Point::new(r.p0.x + p.x, r.p0.y + p.y),
        p1: Point::new(r.p1.x + p.x, r.p1.y + p.y),
    }
}
