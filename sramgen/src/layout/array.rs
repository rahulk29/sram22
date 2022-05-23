use layout21::{
    raw::{BoundBox, Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::PdkLib;

use crate::tech::{colend_cent_gds, corner_gds};

use super::Result;

pub fn draw_array(rows: usize, cols: usize, lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "sram_core".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let corner = corner_gds(lib.pdk.layers())?;
    let corner_bbox = bbox(&corner);
    let colend_cent = colend_cent_gds(lib.pdk.layers())?;
    let colend_bbox = bbox(&colend_cent);

    let xstart = 0;
    let ystart = 0;
    let mut x = xstart;
    let mut y = ystart;

    layout.insts.push(Instance {
        inst_name: "corner_ul".to_string(),
        cell: corner_gds(lib.pdk.layers())?,
        loc: Point::new(x, y),
        reflect_vert: false,
        angle: None,
    });
    x += corner_bbox.width();

    for i in 0..cols {
        layout.insts.push(Instance {
            inst_name: format!("colend_top_{}", i),
            cell: colend_cent.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });
        x += colend_bbox.width();
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

fn bbox(cell: &Ptr<Cell>) -> BoundBox {
    let cell = cell.read().unwrap();
    cell.layout.as_ref().unwrap().bbox()
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;

    #[test]
    fn test_sram_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_array")?;
        draw_array(32, 32, &mut lib)?;

        lib.save_gds()?;

        Ok(())
    }
}
