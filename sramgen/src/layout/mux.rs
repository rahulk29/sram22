use layout21::raw::geom::Dir;
use layout21::raw::{Element, Rect, Shape, Span};
use layout21::{
    raw::{BoundBoxTrait, Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::{
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use crate::layout::array::*;
use crate::layout::route::grid::{Grid, TrackLocator};
use crate::layout::route::Router;
use crate::Result;

pub fn draw_read_mux(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "read_mux".to_string();

    let mut layout = Layout::new(&name);

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(Dir::Horiz)
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

    let mos1 = Instance::builder()
        .inst_name("mos_1")
        .cell(ptx.cell.clone())
        .loc(Point::zero())
        .angle(90f64)
        .build()
        .unwrap();

    let bbox = mos1.bbox();
    layout.insts.push(mos1.clone());

    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    let space = tc.layer("diff").space;

    let mos2 = Instance::builder()
        .inst_name("mos_2")
        .cell(ptx.cell.clone())
        .loc(Point::new(bbox.width() + space, 0))
        .angle(90f64)
        .build()?;
    layout.insts.push(mos2.clone());

    let center = layout.bbox().center();
    let grid = Grid::builder()
        .line(tc.layer("m1").width)
        .space(tc.layer("m1").space)
        .center(center)
        .grid(tc.grid)
        .build()?;

    let bl_lim = mos2.port("sd_0_0").largest_rect(lib.pdk.metal(0)).unwrap();
    let track = grid.get_track_index(Dir::Vert, bl_lim.left(), TrackLocator::StartsBeyond);
    assert!(track >= 3);

    let bbox = layout.bbox().into_rect();
    let mut router = Router::new("read_mux_route", lib.clone());
    let mut traces = Vec::with_capacity(5);

    for i in [-track - 2, -track, -1, 1, track, track + 2] {
        let rect = Rect::span_builder()
            .with(Dir::Horiz, grid.vtrack(i))
            .with(Dir::Vert, Span::new(bbox.bottom(), bbox.top()))
            .build();
        traces.push(router.trace(rect, 1));
        layout.elems.push(Element {
            net: None,
            layer: lib.pdk.metal(1),
            inner: Shape::Rect(rect),
            purpose: layout21::raw::LayerPurpose::Drawing,
        });
    }

    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);

    let src = mos1.port("sd_1_1").largest_rect(m0).unwrap();
    let mut tbr = router.trace(src, 0);
    tbr.place_cursor_centered().horiz_to_trace(&traces[1]).up();

    let src = mos2.port("sd_1_0").largest_rect(m0).unwrap();
    let mut tbr = router.trace(src, 0);
    tbr.place_cursor_centered().horiz_to_trace(&traces[4]).up();

    let src = mos1.port("sd_0_1").largest_rect(m0).unwrap();
    let mut tbr = router.trace(src, 0);
    tbr.place_cursor_centered().horiz_to_trace(&traces[0]).up();

    let src = mos2.port("sd_0_0").largest_rect(m0).unwrap();
    let mut tbr = router.trace(src, 0);
    tbr.place_cursor_centered().horiz_to_trace(&traces[5]).up();

    let br_read_1 = mos1.port("sd_1_0").largest_rect(m0).unwrap();
    let br_read_2 = mos2.port("sd_1_1").largest_rect(m0).unwrap();
    let mut trace = router.trace(br_read_1, 0);
    trace
        .place_cursor_centered()
        .horiz_to(br_read_2.left())
        .contact_up(traces[2].rect());

    let bl_read_1 = mos1.port("sd_0_0").largest_rect(m0).unwrap();
    let bl_read_2 = mos2.port("sd_0_1").largest_rect(m0).unwrap();
    let mut trace = router.trace(bl_read_1, 0);
    trace
        .place_cursor_centered()
        .horiz_to(bl_read_2.left())
        .contact_up(traces[3].rect());

    layout.insts.push(router.finish());

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_read_mux_array(lib: &mut PdkLib, width: usize) -> Result<ArrayedCell> {
    let mux = draw_read_mux(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "read_mux_array".to_string(),
            num: width,
            cell: mux,
            spacing: Some(2_500 * 2),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
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
        .direction(Dir::Horiz)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: 2_000,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    let mos_gnd = Instance::builder()
        .inst_name("mos_1")
        .cell(ptx.cell.clone())
        .angle(90f64)
        .build()?;

    let bbox = mos_gnd.bbox();
    layout.insts.push(mos_gnd);

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(Dir::Horiz)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: 2_000,
            length: 150,
            fingers: 2,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    let space = tc.layer("diff").space;

    let mos_bls = Instance::builder()
        .inst_name("mos_2")
        .cell(ptx.cell.clone())
        .angle(90f64)
        .loc(Point::new(0, bbox.height() + space))
        .build()?;

    layout.insts.push(mos_bls.clone());

    let mut router = Router::new("write_mux_route", lib.clone());

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_write_mux_array(lib: &mut PdkLib, width: usize) -> Result<ArrayedCell> {
    let mux = draw_write_mux(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "write_mux_array".to_string(),
            num: width,
            cell: mux,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_column_read_mux() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux")?;
        draw_read_mux(&mut lib)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_read_mux_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux_array")?;
        draw_read_mux_array(&mut lib, 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux")?;
        draw_write_mux(&mut lib)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux_array")?;
        draw_write_mux_array(&mut lib, 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
