use crate::config::TechConfig;
use crate::error::Result;
use magic_vlsi::units::{Distance, Rect};
use magic_vlsi::{Direction, MagicInstance};

#[derive(Debug, PartialEq, Eq)]
pub struct BusBuilder {
    line: Option<Distance>,
    space: Option<Distance>,
    layer: Option<String>,
    vertical: Option<bool>,
    width: Option<usize>,
    extent1: Option<Distance>,
    extent2: Option<Distance>,
    start: Option<Distance>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Bus {
    line: Distance,
    space: Distance,
    layer: String,
    vertical: bool,
    width: usize,
    extent1: Distance,
    extent2: Distance,
    start: Distance,
}

impl Default for BusBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl BusBuilder {
    pub fn new() -> Self {
        Self {
            line: None,
            space: None,
            layer: None,
            vertical: None,
            width: None,
            extent1: None,
            extent2: None,
            start: None,
        }
    }

    #[inline]
    pub fn is_vertical(&self) -> bool {
        self.vertical.unwrap()
    }

    pub fn align_right(mut self, right: Distance) -> Self {
        assert!(self.is_vertical());
        assert!(self.width.unwrap() > 0);
        self.start = Some(
            right
                - self.line.unwrap() * self.width.unwrap()
                - self.space.unwrap() * (self.width.unwrap() - 1),
        );
        self
    }

    pub fn align_left(mut self, left: Distance) -> Self {
        assert!(self.is_vertical());
        assert!(self.width.unwrap() > 0);
        self.start = Some(left);
        self
    }

    pub fn align_bot(mut self, bot: Distance) -> Self {
        assert!(!self.is_vertical());
        assert!(self.width.unwrap() > 0);
        self.start = Some(bot);
        self
    }

    pub fn align_top(mut self, top: Distance) -> Self {
        assert!(!self.is_vertical());
        assert!(self.width.unwrap() > 0);
        self.start = Some(
            top - self.line.unwrap() * self.width.unwrap()
                - self.space.unwrap() * (self.width.unwrap() - 1),
        );
        self
    }

    pub fn start(mut self, s: Distance) -> Self {
        self.extent1 = Some(s);
        self
    }

    pub fn end(mut self, e: Distance) -> Self {
        self.extent2 = Some(e);
        self
    }

    pub fn width(mut self, w: usize) -> Self {
        self.width = Some(w);
        self
    }

    pub fn dir(mut self, dir: Direction) -> Self {
        self.vertical = Some(matches!(dir, Direction::Up | Direction::Down));
        self
    }

    pub fn tech_layer(mut self, tc: &TechConfig, layer: &str) -> Self {
        self.layer = Some(layer.into());
        self.line = Some(tc.layer(layer).width);
        self.space = Some(tc.layer(layer).space);
        self
    }

    pub fn allow_contact(mut self, tc: &TechConfig, contact: &str, metal_layer: &str) -> Self {
        let layer = self.layer.as_ref().unwrap();
        let line = self.line.unwrap();
        let space = self.space.unwrap();
        let space = [
            space,
            // contact width + enclosure keeps metal layer separated
            tc.layer(contact).width
                + 2 * tc.layer(contact).enclosure(layer)
                + tc.layer(layer).space
                - line,
            // obey min spacing of contacts
            tc.layer(contact).width + tc.layer(contact).space - line,
            // metal layers are sufficiently separated even for adjacent contacts
            tc.layer(contact).width
                + 2 * tc.layer(contact).enclosure(metal_layer)
                + tc.layer(metal_layer).space
                - line,
        ]
        .into_iter()
        .max()
        .unwrap();
        self.space = Some(space);
        self
    }

    pub fn draw(self, m: &mut MagicInstance) -> Result<Bus> {
        let line = self.line.unwrap();
        let space = self.space.unwrap();
        let layer = self.layer.unwrap();
        let vertical = self.vertical.unwrap();
        let width = self.width.unwrap();
        let extent1 = self.extent1.unwrap();
        let extent2 = self.extent2.unwrap();
        let start = self.start.unwrap();

        let bus = Bus {
            line,
            space,
            layer,
            vertical,
            width,
            extent1,
            extent2,
            start,
        };

        bus.draw(m)?;

        Ok(bus)
    }
}

impl Bus {
    fn draw(&self, m: &mut MagicInstance) -> Result<()> {
        assert!(self.extent2 > self.extent1);
        for i in 0..self.width {
            let wire_bbox = self.wire_bbox(i);
            m.paint_box(wire_bbox, &self.layer)?;
        }
        Ok(())
    }

    fn wire_bbox(&self, idx: usize) -> Rect {
        assert!(idx < self.width);
        let length = self.extent2 - self.extent1;
        if self.vertical {
            Rect::ll_wh(
                (self.line + self.space) * idx + self.start,
                self.extent1,
                self.line,
                length,
            )
        } else {
            Rect::ll_wh(
                self.extent1,
                (self.line + self.space) * idx + self.start,
                length,
                self.line,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_contact(
        &self,
        m: &mut MagicInstance,
        tc: &TechConfig,
        idx: usize,
        contact: &str,
        ct_paint: &str,
        layer: &str,
        target: Rect,
    ) -> Result<()> {
        assert!(idx < self.width);
        let wire_bbox = self.wire_bbox(idx);
        let branch_box = if self.vertical {
            let left = std::cmp::min(wire_bbox.left_edge(), target.left_edge());
            let right = std::cmp::max(wire_bbox.right_edge(), target.right_edge());
            let branch_box =
                Rect::lrcyh(left, right, target.center_y(tc.grid), tc.layer(layer).width);
            m.paint_box(branch_box, layer)?;
            branch_box
        } else {
            let bot = std::cmp::min(wire_bbox.bottom_edge(), target.bottom_edge());
            let top = std::cmp::max(wire_bbox.top_edge(), target.top_edge());
            let branch_box = Rect::btcxw(bot, top, target.center_x(tc.grid), tc.layer(layer).width);
            m.paint_box(branch_box, layer)?;
            branch_box
        };

        let contact_region = wire_bbox.overlap(branch_box);
        let contact_box = Rect::ll_wh(
            Distance::zero(),
            Distance::zero(),
            tc.layer(contact).width,
            tc.layer(contact).width,
        );
        let contact_box = contact_box.try_align_center(contact_region, tc.grid);
        m.paint_box(contact_box, ct_paint)?;

        let mut bus_expand_box = contact_box;
        bus_expand_box = bus_expand_box.grow_border(tc.layer(contact).enclosure(&self.layer));
        let extra = std::cmp::max(
            Distance::zero(),
            tc.layer(contact).one_side_enclosure(&self.layer)
                - tc.layer(contact).enclosure(&self.layer),
        );

        if self.vertical {
            bus_expand_box
                .grow(Direction::Up, extra)
                .grow(Direction::Down, extra);
        } else {
            bus_expand_box
                .grow(Direction::Left, extra)
                .grow(Direction::Right, extra);
        }

        m.paint_box(bus_expand_box, &self.layer)?;

        let mut branch_expand_box = contact_box;
        branch_expand_box = branch_expand_box.grow_border(tc.layer(contact).enclosure(layer));
        let extra = std::cmp::max(
            Distance::zero(),
            tc.layer(contact).one_side_enclosure(layer) - tc.layer(contact).enclosure(layer),
        );

        if self.vertical {
            branch_expand_box
                .grow(Direction::Up, extra)
                .grow(Direction::Down, extra);
        } else {
            branch_expand_box
                .grow(Direction::Left, extra)
                .grow(Direction::Right, extra);
        }

        m.paint_box(branch_expand_box, layer)?;
        Ok(())
    }
}
