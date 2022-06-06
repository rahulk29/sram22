use crate::layout::Result;
use layout21::{
    raw::{Cell, Element, Instance, LayerKey, Layout, Point, Rect, Shape},
    utils::Ptr,
};
use pdkprims::{
    geometry::CoarseDirection,
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use super::draw_rect;

pub fn draw_nand2_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let name = "nand2_dec_array".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let nand2 = super::gate::draw_nand2(lib)?;

    // height of 1 sram bitcell
    let spacing = 1580;

    for i in 0..width {
        let mut inst = Instance {
            inst_name: format!("nand_dec_{}", i),
            cell: nand2.clone(),
            loc: Point::new(0, spacing * i as isize),
            reflect_vert: false,
            angle: None,
        };

        if i % 2 == 0 {
            inst.reflect_vert_anchored();
        }

        layout.insts.push(inst);
    }

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;

    #[test]
    fn test_sky130_nand2_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_nand2_dec_array")?;
        draw_nand2_array(&mut lib, 32)?;

        lib.save_gds()?;

        Ok(())
    }
}
