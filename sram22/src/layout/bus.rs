use crate::config::TechConfig;
use crate::error::Result;
use magic_vlsi::units::{Distance, Rect, Vec2};
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
        let length = self.extent2 - self.extent1;

        for i in 0..self.width {
            let rect = if self.vertical {
                Rect::ll_wh(
                    (self.line + self.space) * i + self.start,
                    self.extent1,
                    self.line,
                    length,
                )
            } else {
                Rect::ll_wh(
                    self.extent1,
                    (self.line + self.space) * i + self.start,
                    length,
                    self.line,
                )
            };

            m.paint_box(rect, &self.layer)?;
        }
        Ok(())
    }

    pub fn draw_contact(
        &self,
        m: &mut MagicInstance,
        tc: &TechConfig,
        idx: usize,
        layer: &str,
        pos: Vec2,
    ) -> Result<()> {
        assert!(idx < self.width);
        if self.vertical {
            let llx = self.start + (self.line + self.space) * idx;
            let width = pos.x - llx;
            let rect = Rect::ll_wh(llx, pos.y, width, tc.layer(layer).width);
            m.paint_box(rect, layer)?;
        }

        Ok(())
    }
}
