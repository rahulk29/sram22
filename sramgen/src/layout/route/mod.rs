use pdkprims::contact::ContactParams;
use pdkprims::{LayerIdx, Pdk};

use std::sync::Arc;

use layout21::raw::align::AlignRect;
use layout21::raw::{BoundBoxTrait, Cell, Dir, Instance, Int, LayerKey, Layout, Point, Rect, Span};
use layout21::utils::Ptr;

pub enum VertDir {
    Above,
    Below,
}

pub mod grid;

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
            _ => panic!("No stack for layer index {}", layer),
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

    pub fn new(name: impl Into<String>, pdk: Pdk) -> Self {
        let name = name.into();
        let cell = Cell {
            name: name.clone(),
            abs: None,
            layout: Some(Layout {
                name,
                ..Default::default()
            }),
        };

        Self {
            cfg: Arc::new(RouterConfig::new(pdk)),
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

    #[inline]
    pub fn finish(self) -> Instance {
        Instance::new("__route", self.cell)
    }

    #[inline]
    pub fn cell(&self) -> Instance {
        Instance::new("__route", self.cell.clone())
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
        assert!(width > 0);
        let mut next = Rect::new(Point::new(0, 0), Point::new(width, width));
        next.align_centers_gridded(self.rect.bbox(), grid);
        self.rect = next;
    }
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

pub enum ContactBounds {
    /// Place a minimum size contact, even if it requires expanding all layers.
    Minimum,
    /// Fit the contact into the given rectangle on all layers.
    Fit(Rect),
    /// Fit the contact into the given rectangle on the specified layer.
    /// All other layers can be expanded.
    FitOne(LayerKey, Rect),
    /// Fill contacts within the given size in the given direction on the given layer
    FillDir {
        dir: Dir,
        size: Int,
        layer: LayerKey,
    },
}

impl Trace {
    #[inline]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    #[inline]
    pub fn horiz_to_trace(&mut self, other: &Self) -> &mut Self {
        let target = other
            .rect
            .edge_farther_from(self.rect.center().x, Dir::Horiz);
        self.horiz_to(target)
    }

    #[inline]
    pub fn horiz_to_rect(&mut self, rect: Rect) -> &mut Self {
        let target = rect.edge_farther_from(self.rect.center().x, Dir::Horiz);
        self.horiz_to(target)
    }

    #[inline]
    pub fn vert_to_trace(&mut self, other: &Self) -> &mut Self {
        let target = other
            .rect
            .edge_farther_from(self.rect.center().y, Dir::Vert);
        self.vert_to(target)
    }

    #[inline]
    pub fn vert_to_rect(&mut self, rect: Rect) -> &mut Self {
        let target = rect.edge_farther_from(self.rect.center().y, Dir::Vert);
        self.vert_to(target)
    }

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
        let src_edge = cr.edge_farther_from(x, dir);
        let l_span = Span::new(x, src_edge);

        let rect = Rect::span_builder()
            .with(dir, l_span)
            .with(!dir, x_span)
            .build();
        self.add_rect(rect);
        self.rect = rect;

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

    pub fn place_cursor_centered(&mut self) -> &mut Self {
        let mut rect = Rect::new(Point::new(0, 0), Point::new(self.width, self.width));
        rect.align_centers_gridded(self.rect.into(), self.grid());
        self.cursor = Some(Cursor::new(rect));
        assert_eq!(rect.width(), self.width);
        assert_eq!(rect.height(), self.width);
        self
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
        self.place_cursor_centered();
        self
    }

    pub fn up(&mut self) -> &mut Self {
        let rect = self.cursor_rect();
        self.contact_on(rect, VertDir::Above, ContactBounds::Minimum);
        self.layer += 1;
        self
    }

    pub fn down(&mut self) -> &mut Self {
        let rect = self.cursor_rect();
        self.contact_on(rect, VertDir::Below, ContactBounds::Minimum);
        self.layer -= 1;
        self
    }

    pub fn contact_down(&mut self, rect: Rect) -> &mut Self {
        let intersect = Rect::from(self.rect.intersection(&rect.bbox()));
        self.contact_on(
            intersect,
            VertDir::Below,
            ContactBounds::FitOne(self.cfg.layerkey(self.layer), intersect),
        );
        self
    }

    pub fn contact_up(&mut self, rect: Rect) -> &mut Self {
        let intersect = Rect::from(self.rect.intersection(&rect.bbox()));
        self.contact_on(
            intersect,
            VertDir::Above,
            ContactBounds::FitOne(self.cfg.layerkey(self.layer), intersect),
        );
        self
    }

    pub fn increment_layer(&mut self) -> &mut Self {
        self.layer += 1;
        self
    }

    pub fn decrement_layer(&mut self) -> &mut Self {
        self.layer -= 1;
        self
    }

    /// If bounded, place contacts only within `rect`.
    pub fn contact_on(
        &mut self,
        rect: Rect,
        vert_dir: VertDir,
        bounds: ContactBounds,
    ) -> &mut Self {
        let stack_layer = match vert_dir {
            VertDir::Above => self.layer + 1,
            VertDir::Below => self.layer,
        };
        let min_params = ContactParams::builder()
            .rows(1)
            .cols(1)
            .dir(rect.longer_dir())
            .stack(self.cfg.stack(stack_layer).to_string())
            .build()
            .unwrap();
        let ct = match bounds {
            ContactBounds::Minimum => self.cfg.pdk.get_contact(&min_params),
            ContactBounds::FitOne(layerkey, rect) => self
                .cfg
                .pdk
                .get_contact_within(self.cfg.stack(stack_layer), layerkey, rect)
                .or_else(|| Some(self.cfg.pdk.get_contact(&min_params)))
                .unwrap(),
            ContactBounds::FillDir { dir, layer, size } => self
                .cfg
                .pdk
                .get_contact_sized(self.cfg.stack(stack_layer), dir, layer, size)
                .or_else(|| Some(self.cfg.pdk.get_contact(&min_params)))
                .unwrap(),
            _ => todo!(),
        };

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
