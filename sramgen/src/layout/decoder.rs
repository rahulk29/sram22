use crate::{layout::Result, tech::BITCELL_HEIGHT};
use layout21::{
    raw::{Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::PdkLib;

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

pub fn draw_inv_dec_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let name = "inv_dec_array".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let arr_cell = super::gate::draw_inv_dec(lib)?;

    for i in 0..width {
        let mut inst = Instance {
            inst_name: format!("inv_dec_{}", i),
            cell: arr_cell.clone(),
            loc: Point::new(0, BITCELL_HEIGHT * i as isize),
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

    #[test]
    fn test_sky130_inv_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_inv_dec_array")?;
        draw_inv_dec_array(&mut lib, 32)?;

        lib.save_gds()?;

        Ok(())
    }
}
