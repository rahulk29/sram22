use layout21::{
    raw::{BoundBox, Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::PdkLib;

use crate::{bbox, tech::*};

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
    let colend_cent = colend_cent_gds(lib.pdk.layers())?;
    let colend_cent_bbox = bbox(&colend_cent);
    let colend = colend_gds(lib.pdk.layers())?;
    let colend_bbox = bbox(&colend);

    let bitcell = sram_sp_cell_gds(lib.pdk.layers())?;
    let bitcell_bbox = bbox(&bitcell);
    let rowend = rowend_gds(lib.pdk.layers())?;
    let rowend_bbox = bbox(&rowend);
    let rowend_width = 1300;
    let row_height = 1580;
    let colend_width = 1200;
    let colend_height = 2055;
    let wlstrap = wlstrap_gds(lib.pdk.layers())?;
    let wlstrap_bbox = bbox(&wlstrap);
    let wlstrap_p = wlstrap_p_gds(lib.pdk.layers())?;
    let wlstrap_p_bbox = bbox(&wlstrap_p);
    assert_eq!(colend_bbox.width(), 1200);

    println!("rowend bbox: {:?}", rowend_bbox);

    let xstart = 0;
    let ystart = 0;
    let mut x = xstart;
    let mut y = ystart;

    layout.insts.push(Instance {
        inst_name: "corner_ul".to_string(),
        cell: corner.clone(),
        loc: Point::new(x, y),
        reflect_vert: false,
        angle: None,
    });
    x += rowend_width;

    layout.insts.push(Instance {
        inst_name: "colend_top_0".to_string(),
        cell: colend.clone(),
        loc: Point::new(x, y),
        reflect_vert: false,
        angle: None,
    });
    x += colend_width;

    for i in 1..cols {
        layout.insts.push(Instance {
            inst_name: format!("colend_cent_top_{}", i),
            cell: colend_cent.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });
        x += colend_cent_bbox.width();

        layout.insts.push(Instance {
            inst_name: format!("colend_top_{}", i),
            cell: colend.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });
        x += colend_width;
    }

    layout.insts.push(Instance {
        inst_name: "corner_ur".to_string(),
        cell: corner.clone(),
        loc: Point::new(x, y),
        reflect_vert: false,
        angle: None,
    });

    y -= row_height;

    for r in 0..rows {
        x = xstart;
        layout.insts.push(Instance {
            inst_name: format!("rowend_l_{}", r),
            cell: rowend.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });
        x += rowend_width;

        layout.insts.push(Instance {
            inst_name: format!("cell_{}_0", r),
            cell: bitcell.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });
        x += bitcell_bbox.width();

        for c in 1..cols {
            layout.insts.push(Instance {
                inst_name: format!("wlstrap_{}_{}", r, c),
                cell: wlstrap.clone(),
                loc: Point::new(x, y),
                reflect_vert: false,
                angle: None,
            });
            x += wlstrap_bbox.width();

            layout.insts.push(Instance {
                inst_name: format!("cell_{}_{}", r, c),
                cell: bitcell.clone(),
                loc: Point::new(x, y),
                reflect_vert: false,
                angle: None,
            });
            x += bitcell_bbox.width();
        }

        layout.insts.push(Instance {
            inst_name: format!("rowend_r_{}", r),
            cell: rowend.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });

        y -= row_height;
    }

    y -= colend_height - row_height;

    x = xstart;
    layout.insts.push(Instance {
        inst_name: "corner_bl".to_string(),
        cell: corner.clone(),
        loc: Point::new(x, y),
        reflect_vert: false,
        angle: None,
    });
    x += rowend_width;

    layout.insts.push(Instance {
        inst_name: "colend_bot_0".to_string(),
        cell: colend.clone(),
        loc: Point::new(x, y),
        reflect_vert: false,
        angle: None,
    });
    x += colend_width;

    for i in 1..cols {
        layout.insts.push(Instance {
            inst_name: format!("colend_cent_bot_{}", i),
            cell: colend_cent.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });
        x += colend_cent_bbox.width();

        layout.insts.push(Instance {
            inst_name: format!("colend_bot_{}", i),
            cell: colend.clone(),
            loc: Point::new(x, y),
            reflect_vert: false,
            angle: None,
        });
        x += colend_width;
    }

    layout.insts.push(Instance {
        inst_name: "corner_br".to_string(),
        cell: corner.clone(),
        loc: Point::new(x, y),
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
