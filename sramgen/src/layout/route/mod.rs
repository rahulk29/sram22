use pdkprims::{contact::ContactParams, LayerIdx, Pdk, PdkLib};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use layout21::{
    raw::{
        align::AlignRect, BoundBoxTrait, Cell, Dir, Instance, Int, LayerKey, Layout, Point, Rect,
        Span,
    },
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
            cursor: None,
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

pub struct Trace {
    width: Int,
    layer: LayerIdx,
    rect: Rect,
    cursor: Option<Cursor>,
    cfg: Arc<RouterConfig>,
    cell: Ptr<Cell>,

    id: usize,
    ctr: usize,
}

struct Cursor {
    rect: Rect,
}

impl Cursor {
    #[inline]
    fn new(rect: Rect) -> Self {
        Self { rect }
    }

    #[inline]
    fn rect(&self) -> Rect {
        self.rect
    }

    #[inline]
    fn move_to(&mut self, rect: Rect) {
        self.rect = rect;
    }

    fn resize(&mut self, width: Int, grid: Int) {
        let mut next = Rect::new(Point::new(0, width), Point::new(0, width));
        next.align_centers_gridded(self.rect.bbox(), grid);
        self.rect = next;
    }
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
    #[inline]
    pub fn horiz_to(&mut self, x: Int) -> &mut Self {
        self.draw_to(x, Dir::Horiz)
    }
    #[inline]
    pub fn vert_to(&mut self, x: Int) -> &mut Self {
        self.draw_to(x, Dir::Vert)
    }

    pub fn draw_to(&mut self, x: Int, dir: Dir) -> &mut Self {
        let cr = self.cursor_rect();

        let x_span = cr.span(!dir);
        let src_edge = cr.edge_closer_to(x, dir);
        let l_span = Span::new(x, src_edge);

        let rect = Rect::span_builder()
            .with(dir, l_span)
            .with(!dir, x_span)
            .build();
        self.add_rect(rect);

        let l_span = if x > src_edge {
            Span::new(x - cr.span(dir).length(), x)
        } else {
            Span::new(x, x + cr.span(dir).length())
        };

        let nc = Rect::span_builder()
            .with(dir, l_span)
            .with(!dir, x_span)
            .build();
        self.move_cursor_to(nc);

        self
    }

    pub fn set_width(&mut self, width: Int) -> &mut Self {
        self.width = width;
        let grid = self.grid();
        if let Some(ref mut cursor) = self.cursor {
            cursor.resize(width, grid);
        }
        self
    }

    #[inline]
    pub fn set_min_width(&mut self) -> &mut Self {
        self.set_width(self.cfg.line(self.layer))
    }

    pub fn place_cursor(&mut self, dir: Dir, pos: bool) -> &mut Self {
        let x_span =
            Span::from_center_span_gridded(self.rect.span(!dir).center(), self.width, self.grid());
        let edge_1 = self.rect.span(dir).edge(pos);
        let edge_2 = if pos {
            edge_1 - self.width
        } else {
            edge_1 + self.width
        };
        let l_span = Span::new(edge_1, edge_2);
        let rect = Rect::span_builder()
            .with(dir, l_span)
            .with(!dir, x_span)
            .build();
        self.cursor = Some(Cursor::new(rect));
        self
    }

    fn cursor_rect(&self) -> Rect {
        self.cursor.as_ref().unwrap().rect()
    }

    fn move_cursor_to(&mut self, rect: Rect) {
        self.cursor.as_mut().unwrap().move_to(rect);
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

    pub fn s_bend(&mut self, target: Rect, dir: Dir) -> &mut Self {
        use std::cmp::{max, min};

        let bot = min(self.rect.lower_edge(!dir), target.lower_edge(!dir));
        let top = max(self.rect.upper_edge(!dir), target.upper_edge(!dir));

        let x_span = Span::new(bot, top);

        // An S-bend does not make sense if the spans overlap
        assert!(!self.rect.span(dir).intersects(&target.span(dir)));

        let (inner_left, inner_right) = if self.rect.lower_edge(dir) < target.lower_edge(dir) {
            (self.rect.upper_edge(dir), target.lower_edge(dir))
        } else {
            (target.upper_edge(dir), self.rect.lower_edge(dir))
        };

        let inner_span = Span::new(inner_left, inner_right);

        let mid = Span::from_center_span_gridded(inner_span.center(), self.width, self.grid());
        let mid = Rect::span_builder()
            .with(dir, mid)
            .with(!dir, x_span)
            .build();
        self.add_rect(mid);

        let src = Rect::span_builder()
            .with(
                dir,
                Span::new(self.rect.upper_edge(dir), mid.lower_edge(dir)),
            )
            .with(!dir, self.rect.span(!dir))
            .build();
        self.add_rect(src);

        let dst = Rect::span_builder()
            .with(dir, Span::new(mid.upper_edge(dir), target.lower_edge(dir)))
            .with(!dir, target.span(!dir))
            .build();
        self.add_rect(dst);

        self.rect = target;
        self
    }

    pub fn up(&mut self) -> &mut Self {
        self.layer += 1;
        // important: increment self.layer, then place the contact
        let rect = self.cursor_rect();
        self.inner_contact_on(rect);
        self
    }

    pub fn down(&mut self) -> &mut Self {
        let rect = self.cursor_rect();
        // important: place the contact, then decrement self.layer
        self.inner_contact_on(rect);
        self.layer -= 1;
        self
    }

    pub fn contact_down(&mut self, rect: Rect) -> &mut Self {
        let intersect = Rect::from(self.rect.intersection(&rect.bbox()));
        self.inner_contact_on(intersect);
        self
    }

    fn inner_contact_on(&mut self, rect: Rect) {
        let ct = self.cfg.pdk.get_contact(
            &ContactParams::builder()
                .rows(1)
                .cols(1)
                .dir(rect.longer_dir())
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

        inst.align_centers_gridded(rect.into(), self.grid());
        self.add_inst(inst);
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
