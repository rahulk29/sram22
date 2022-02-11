use crate::error::Result;
use magic_vlsi::{units::Rect, MagicInstance};

pub struct LayoutCell {
    pub name: String,
    pub bbox: Rect,
    pub ports: Vec<LayoutPort>,
}

pub struct LayoutPort {
    pub name: String,
    pub bbox: Rect,
    pub layer: String,
}

impl LayoutCell {
    pub fn load(m: &mut MagicInstance, cell: &str) -> Result<LayoutCell> {
        m.load(cell)?;
        m.select_top_cell()?;
        let bbox = m.select_bbox()?;

        let mut idx = m.port_first()?;

        let mut ports = Vec::with_capacity(m.port_last()? as usize);

        while idx != -1 {
            let name = m.port_index_name(idx)?;
            m.findlabel(&name)?;
            m.select_visible()?;
            let bbox = m.select_bbox()?;
            let layer = m.label_layer()?;

            ports.push(LayoutPort { name, bbox, layer });
            idx = m.port_next(idx)?;
        }

        Ok(LayoutCell {
            name: cell.to_string(),
            bbox,
            ports,
        })
    }
}
