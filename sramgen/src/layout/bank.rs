use layout21::{
    raw::{AlignMode, Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::PdkLib;

use crate::{bbox, layout::grid::GridCells, tech::*};

use super::{
    array::draw_array,
    decoder::{draw_inv_dec_array, draw_nand2_array},
    Result,
};

pub fn draw_sram_bank(rows: usize, cols: usize, lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "sram_bank".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let core = draw_array(rows, cols, lib)?;
    let nand2_dec = draw_nand2_array(lib, rows)?;
    let inv_dec = draw_inv_dec_array(lib, rows)?;

    let core = Instance {
        cell: core,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "core".to_string(),
        reflect_vert: false,
    };

    let mut nand2_dec = Instance {
        cell: nand2_dec,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "nand2_dec_array".to_string(),
        reflect_vert: false,
    };

    let mut inv_dec = Instance {
        cell: inv_dec,
        inst_name: "inv_dec_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    inv_dec
        .align(&core, AlignMode::ToTheLeft, 1_000)
        .align(&core, AlignMode::CenterVertical, 0);
    nand2_dec
        .align(&inv_dec, AlignMode::ToTheLeft, 1_000)
        .align(&core, AlignMode::CenterVertical, 0);

    layout.insts.push(core);
    layout.insts.push(nand2_dec);
    layout.insts.push(inv_dec);

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
    fn test_sram_bank() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank")?;
        draw_sram_bank(32, 32, &mut lib)?;

        lib.save_gds()?;

        Ok(())
    }
}
