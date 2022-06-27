use pdkprims::{LayerIdx, Pdk};
use serde::{Deserialize, Serialize};
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use layout21::{
    raw::{Cell, Int, LayerKey, Layout, Point, Rect},
    utils::Ptr,
};

pub struct RouterConfig {
    pub(crate) pdk: Pdk,
}

impl RouterConfig {
    pub fn new(pdk: Pdk) -> Self {
        Self { pdk }
    }

    #[inline]
    pub fn line(&self, layer: LayerIdx) -> Int {
        let tc = self.pdk.config.read().unwrap();
        tc.layer(self.pdk.metal_name(layer)).width
    }

    #[inline]
    pub fn space(&self, layer: LayerIdx) -> Int {
        let tc = self.pdk.config.read().unwrap();
        tc.layer(self.pdk.metal_name(layer)).space
    }

    #[inline]
    pub fn grid(&self) -> Int {
        let tc = self.pdk.config.read().unwrap();
        tc.grid
    }

    #[inline]
    pub fn layerkey(&self, layer: LayerIdx) -> LayerKey {
        self.pdk.metal(layer)
    }
}

pub struct Router {
    cfg: Arc<RouterConfig>,
    cell: Ptr<Cell>,
}

impl Router {
    pub fn new(pdk: Pdk) -> Self {
        let cell = Cell {
            name: "router".to_string(),
            abs: None,
            layout: Some(Layout {
                name: "router".to_string(),
                ..Default::default()
            }),
        };

        Self {
            cfg: Arc::new(RouterConfig::new(pdk)),
            cell: Ptr::new(cell),
        }
    }

    pub fn trace(&self, pin: Rect, layer: LayerIdx) -> Trace {
        Trace {
            width: self.cfg.line(layer),
            layer,
            cfg: Arc::clone(&self.cfg),
            rect: pin,
            dir: TraceDir::None,
            cell: Ptr::clone(&self.cell),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TraceDir {
    None,
    Up,
    Down,
    Left,
    Right,
}

pub struct Trace {
    width: Int,
    layer: LayerIdx,
    rect: Rect,
    dir: TraceDir,
    cfg: Arc<RouterConfig>,
    cell: Ptr<Cell>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BusDir {
    Horizontal,
    Vertical,
}

pub struct Bus {
    dir: BusDir,
    layer: LayerIdx,
    tracks: usize,
    line: Int,
    space: Int,
}

// Rounds a to the nearest multiple of b
#[inline]
fn round(a: Int, b: Int) -> Int {
    assert!(b > 0);
    let min = (a / b) * b;
    let max = min + b;
    if a - min < max - a {
        min
    } else {
        max
    }
}

pub fn gridded_center_span(center: Int, span: Int, grid: Int) -> (Int, Int) {
    // Span must be a positive multiple of the grid size
    assert!(span > 0);
    assert!(grid > 0);
    assert!(span % grid == 0);

    let xmin = round(center - span / 2, grid);
    let xmax = xmin + span;

    assert!(xmax - xmin == span);

    (xmin, xmax)
}

impl Trace {
    pub fn horiz(&mut self, x: Int) -> &mut Self {
        let next_dir = if x > self.rect.p1.x {
            TraceDir::Right
        } else if x < self.rect.p0.x {
            TraceDir::Left
        } else {
            return self;
        };

        let (p0, p1) = match self.dir {
            TraceDir::Up => (
                Point::new(self.rect.p0.x, self.rect.p1.y - self.width),
                Point::new(x, self.rect.p1.y),
            ),
            TraceDir::Down => (
                Point::new(self.rect.p0.x, self.rect.p0.y),
                Point::new(x, self.rect.p0.y + self.width),
            ),
            TraceDir::Left | TraceDir::Right | TraceDir::None => {
                let (y_min, y_max) = self
                    .cfg
                    .pdk
                    .gridded_center_span(self.rect.center().y, self.width);
                let (x_min, x_max) = if self.dir == TraceDir::Left {
                    (x, self.rect.p0.x)
                } else {
                    (self.rect.p1.x, x)
                };

                (Point::new(x_min, y_min), Point::new(x_max, y_max))
            }
        };

        self.dir = next_dir;

        let rect = rect_from_corners(p0, p1);
        self.rect = rect;
        self.add_rect(rect);

        self
    }

    fn add_rect(&self, rect: Rect) {
        use layout21::raw::{Element, LayerPurpose, Shape};
        let mut cell = self.cell.write().unwrap();
        let layout = cell.layout.as_mut().unwrap();
        layout.elems.push(Element {
            layer: self.cfg.layerkey(self.layer),
            net: None,
            purpose: LayerPurpose::Drawing,
            inner: Shape::Rect(rect),
        });
    }

    pub fn s_bend(&mut self, target: Point, dir: TraceDir) -> &mut Self {
        self
    }

    pub fn vert(&mut self, x: Int) -> &mut Self {
        self
    }

    pub fn up(&mut self) -> &mut Self {
        self
    }

    pub fn down(&mut self) -> &mut Self {
        self
    }
}

fn rect_from_corners(p0: Point, p1: Point) -> Rect {
    use std::cmp::max;
    use std::cmp::min;

    Rect {
        p0: Point::new(min(p0.x, p1.x), min(p0.y, p1.y)),
        p1: Point::new(max(p0.x, p1.x), max(p0.y, p1.y)),
    }
}
