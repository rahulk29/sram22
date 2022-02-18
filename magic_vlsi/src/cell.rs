use std::sync::Arc;

use crate::{
    units::{Rect, Vec2},
    Direction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutCell {
    pub name: String,
    pub bbox: Rect,
    pub ports: Vec<LayoutPort>,
}

pub type LayoutCellRef = Arc<LayoutCell>;

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPort {
    pub name: String,
    pub bbox: Rect,
    pub layer: String,
}

pub type InstanceCellRef = Arc<InstanceCell>;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceCell {
    pub ll: Vec2,
    pub cell: LayoutCellRef,
    /// The name of the cell instance, which may be different from the name of the layout cell.
    pub name: String,

    sideways: bool,
    upside_down: bool,
}

impl InstanceCell {
    #[inline]
    pub(crate) fn sideways(&mut self) {
        self.sideways = !self.sideways;
    }

    #[inline]
    pub(crate) fn upside_down(&mut self) {
        self.upside_down = !self.upside_down;
    }

    pub fn new(ll: Vec2, cell: LayoutCellRef, name: String) -> Self {
        Self {
            ll,
            cell,
            name,
            sideways: false,
            upside_down: false,
        }
    }

    pub fn port(&self, name: &str) -> &LayoutPort {
        self.cell
            .ports
            .iter()
            .find(|&x| x.name == name)
            .unwrap_or_else(|| panic!("port not found: {}", name))
    }

    fn to_global(&self, bbox: Rect) -> Rect {
        let mut tmp = bbox;

        if self.sideways {
            tmp = Rect::ll_wh(
                self.cell.bbox.ur.x - tmp.ur.x + self.cell.bbox.ll.x,
                tmp.ll.y,
                tmp.width(),
                tmp.height(),
            );
        }

        if self.upside_down {
            tmp = Rect::ur_wh(
                tmp.ur.x,
                self.cell.bbox.ur.y - tmp.ll.y + self.cell.bbox.ll.y,
                tmp.width(),
                tmp.height(),
            );
        }

        tmp.translate(Direction::Down, self.cell.bbox.bottom_edge() - self.ll.y)
            .translate(Direction::Left, self.cell.bbox.left_edge() - self.ll.x);

        assert_eq!(tmp.width(), bbox.width());
        assert_eq!(tmp.height(), bbox.height());

        tmp
    }

    #[inline]
    pub fn port_bbox(&self, name: &str) -> Rect {
        self.to_global(self.port(name).bbox)
    }

    #[inline]
    pub fn bbox(&self) -> Rect {
        self.to_global(self.cell.bbox)
    }
}

#[cfg(test)]
mod tests {
    use crate::units::Distance;

    use super::*;

    fn empty_layout_cell() -> LayoutCellRef {
        Arc::new(LayoutCell {
            name: "test_cell".to_string(),
            bbox: Rect::ll_wh(
                Distance::from_um(-1),
                Distance::from_um(-1),
                Distance::from_um(2),
                Distance::from_um(2),
            ),
            ports: vec![],
        })
    }

    #[test]
    fn test_to_global() {
        let mut cell = InstanceCell::new(
            Vec2::from_um(11, 12),
            empty_layout_cell(),
            "test_icell".to_string(),
        );
        let expected = Rect::ll_wh(
            Distance::from_um(11),
            Distance::from_um(12),
            Distance::from_um(2),
            Distance::from_um(2),
        );
        assert_eq!(cell.to_global(cell.cell.bbox), expected);

        cell.sideways();
        assert_eq!(cell.to_global(cell.cell.bbox), expected);

        cell.upside_down();
        assert_eq!(cell.to_global(cell.cell.bbox), expected);

        cell.sideways();
        assert_eq!(cell.to_global(cell.cell.bbox), expected);
    }
}
