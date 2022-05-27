use layout21::{
    raw::{Cell, Element, Instance, LayerKey, LayerPurpose, Layout, Point, Rect, Shape},
    utils::Ptr,
};
use pdkprims::{
    geometry::CoarseDirection,
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use crate::tech::sram_sp_cell_gds;

mod array;
mod gate;
mod mux;
mod precharge;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn draw_bitcell(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "t_bitcell".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let tc = lib.pdk.config.read().unwrap();

    layout.insts.push(Instance {
        inst_name: "mcell".to_string(),
        cell: sram_sp_cell_gds(lib.pdk.layers())?,
        loc: Point::new(0, 0),
        reflect_vert: false,
        angle: None,
    });

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_rect(r: Rect, layer: LayerKey) -> Element {
    Element {
        net: None,
        layer,
        inner: Shape::Rect(r),
        purpose: LayerPurpose::Drawing,
    }
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;
    use crate::layout::Result;

    #[test]
    fn test_sky130_bitcell() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_bitcell")?;
        draw_bitcell(&mut lib)?;

        lib.save_gds()?;

        Ok(())
    }
}
