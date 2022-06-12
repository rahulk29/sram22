use layout21::{
    raw::{Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::{
    geometry::CoarseDirection,
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use crate::layout::array::*;
use crate::Result;

pub fn draw_read_mux(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "read_mux".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(CoarseDirection::Horizontal)
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_200,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_200,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    let mos = Instance {
        inst_name: "mos_1".to_string(),
        cell: ptx.cell.clone(),
        loc: Point::new(0, 0),
        angle: Some(90f64),
        reflect_vert: false,
    };
    let bbox = mos.bbox();
    assert_eq!(bbox.width(), 1040);
    layout.insts.push(mos);

    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    let space = tc.layer("diff").space;
    println!("bbox width: {}", bbox.width());

    layout.insts.push(Instance {
        inst_name: "mos_2".to_string(),
        cell: ptx.cell.clone(),
        loc: Point::new(bbox.width() + space, 0),
        angle: Some(90f64),
        reflect_vert: false,
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

pub fn draw_read_mux_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let mux = draw_read_mux(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "read_mux_array".to_string(),
            num: width,
            cell: mux,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: CoarseDirection::Horizontal,
        },
        lib,
    )
}

pub fn draw_write_mux(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "write_mux".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(CoarseDirection::Horizontal)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: 1_200,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    let mos = Instance {
        inst_name: "mos_1".to_string(),
        cell: ptx.cell.clone(),
        loc: Point::new(0, 0),
        angle: Some(90f64),
        reflect_vert: false,
    };
    let bbox = mos.bbox();
    layout.insts.push(mos);

    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    let space = tc.layer("diff").space;

    layout.insts.push(Instance {
        inst_name: "mos_2".to_string(),
        cell: ptx.cell.clone(),
        loc: Point::new(0, bbox.height() + space),
        angle: Some(90f64),
        reflect_vert: false,
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

pub fn draw_write_mux_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let mux = draw_write_mux(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "write_mux_array".to_string(),
            num: width,
            cell: mux,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: CoarseDirection::Horizontal,
        },
        lib,
    )
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;

    #[test]
    fn test_sky130_column_read_mux() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux")?;
        draw_read_mux(&mut lib)?;

        lib.save_gds()?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_read_mux_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux_array")?;
        draw_read_mux_array(&mut lib, 32)?;

        lib.save_gds()?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux")?;
        draw_write_mux(&mut lib)?;

        lib.save_gds()?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux_array")?;
        draw_write_mux_array(&mut lib, 32)?;

        lib.save_gds()?;

        Ok(())
    }
}
