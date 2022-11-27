use crate::config::guard_ring::RingParams;
use derive_builder::Builder;
use layout21::raw::{BoundBox, BoundBoxTrait, Int, Point, Rect, Span};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Builder)]
pub struct Ring {
    top: Rect,
    bottom: Rect,
    left: Rect,
    right: Rect,
}

impl Ring {
    #[inline]
    pub fn builder() -> RingBuilder {
        RingBuilder::default()
    }

    #[inline]
    pub fn h_rects(&self) -> [Rect; 2] {
        [self.bottom, self.top]
    }

    #[inline]
    pub fn v_rects(&self) -> [Rect; 2] {
        [self.left, self.right]
    }

    #[inline]
    pub fn rects(&self) -> [Rect; 4] {
        [self.left, self.bottom, self.right, self.top]
    }

    /// The area within the ring.
    ///
    /// Does not include the ring itself.
    #[inline]
    pub fn inner_enclosure(&self) -> Rect {
        Rect::new(
            Point::new(self.left.p1.x, self.bottom.p1.y),
            Point::new(self.right.p0.x, self.top.p0.y),
        )
    }

    /// The bounding box of the ring.
    ///
    /// Includes the area within the ring, as well as the ring itself.
    #[inline]
    pub fn outer_enclosure(&self) -> Rect {
        Rect::new(self.bottom.p0, self.top.p1)
    }

    #[inline]
    pub fn left(&self) -> Rect {
        self.left
    }
    #[inline]
    pub fn right(&self) -> Rect {
        self.right
    }
    #[inline]
    pub fn bottom(&self) -> Rect {
        self.bottom
    }
    #[inline]
    pub fn top(&self) -> Rect {
        self.top
    }
}

impl BoundBoxTrait for Ring {
    #[inline]
    fn bbox(&self) -> BoundBox {
        BoundBox::from_points(&self.bottom.p0, &self.top.p1)
    }
}

pub fn draw_ring(params: RingParams) -> Ring {
    let RingParams {
        enclosure,
        h_width,
        v_width,
    } = params;

    let t_span = Span::new(enclosure.top(), enclosure.top() + h_width);
    let b_span = Span::new(enclosure.bottom() - h_width, enclosure.bottom());

    let l_span = Span::new(enclosure.left() - v_width, enclosure.left());
    let r_span = Span::new(enclosure.right(), enclosure.right() + v_width);

    let v_span = Span::new(b_span.start(), t_span.stop());
    let h_span = Span::new(l_span.start(), r_span.stop());

    let left = Rect::from_spans(l_span, v_span);
    let right = Rect::from_spans(r_span, v_span);
    let bottom = Rect::from_spans(h_span, b_span);
    let top = Rect::from_spans(h_span, t_span);

    Ring {
        top,
        bottom,
        right,
        left,
    }
}
