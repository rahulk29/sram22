use pdkprims::{contact::ContactParams, LayerIdx, Pdk, PdkLib};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use layout21::{
    raw::{align::AlignRect, BoundBoxTrait, Cell, Instance, Int, LayerKey, Layout, Point, Rect},
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

    fn stack(&self, layer: LayerIdx) -> &str {
        match layer {
            1 => "viali",
            2 => "via1",
            3 => "via2",
            _ => todo!(),
        }
    }
}

pub struct Router {
    cfg: Arc<RouterConfig>,
    cell: Ptr<Cell>,
    ctr: usize,
}

impl Router {
    #[inline]
    pub fn cfg(&self) -> Arc<RouterConfig> {
        Arc::clone(&self.cfg)
    }

    pub fn new(pdklib: PdkLib) -> Self {
        let cell = Cell {
            name: "router".to_string(),
            abs: None,
            layout: Some(Layout {
                name: "router".to_string(),
                ..Default::default()
            }),
        };

        Self {
            cfg: Arc::new(RouterConfig::new(pdklib.pdk)),
            cell: Ptr::new(cell),
            ctr: 0,
        }
    }

    pub fn trace(&mut self, pin: Rect, layer: LayerIdx) -> Trace {
        self.ctr += 1;
        let trace = Trace {
            width: self.cfg.line(layer),
            layer,
            cfg: Arc::clone(&self.cfg),
            rect: pin,
            dir: TraceDir::None,
            cell: Ptr::clone(&self.cell),
            id: self.ctr,
            ctr: 0,
        };
        trace.add_rect(pin);
        trace
    }

    pub fn finish(self) -> Instance {
        Instance {
            inst_name: "__route".to_string(),
            cell: self.cell,
            loc: Point::new(0, 0),
            angle: None,
            reflect_vert: false,
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

    id: usize,
    ctr: usize,
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

    pub fn down_on(&mut self, rect: Rect) -> &mut Self {
        let intersect = self.rect.intersection(&rect.bbox());
        let ct = self.cfg.pdk.get_contact(
            &ContactParams::builder()
                .rows(1)
                .cols(1)
                .stack(self.cfg.stack(self.layer).to_string())
                .build()
                .unwrap(),
        );

        self.ctr += 1;

        let mut inst = Instance {
            inst_name: format!("contact_{}_{}", self.id, self.ctr),
            cell: Ptr::clone(&ct.cell),
            loc: Point::new(0, 0),
            angle: None,
            reflect_vert: false,
        };

        inst.align_centers_gridded(intersect.into(), self.grid());
        self.add_inst(inst);

        self
    }

    fn grid(&self) -> Int {
        self.cfg.pdk.config().read().unwrap().grid
    }

    fn add_inst(&self, inst: Instance) {
        let mut cell = self.cell.write().unwrap();
        cell.layout.as_mut().unwrap().insts.push(inst);
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
