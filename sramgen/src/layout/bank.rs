use layout21::raw::align::AlignRect;
use layout21::raw::geom::Rect;
use layout21::{
    raw::{BoundBoxTrait, Cell, Instance, Layout, Point, Span},
    utils::Ptr,
};
use pdkprims::PdkLib;

use crate::layout::route::Router;

use super::{
    array::draw_array,
    decoder::{draw_inv_dec_array, draw_nand2_array},
    dff::draw_dff_array,
    mux::{draw_read_mux_array, draw_write_mux_array},
    precharge::draw_precharge_array,
    sense_amp::draw_sense_amp_array,
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

    assert_eq!(cols % 2, 0);

    let grid = { lib.pdk.config().read().unwrap().grid };

    let core = draw_array(rows, cols, lib)?;
    let nand2_dec = draw_nand2_array(lib, rows)?;
    let inv_dec = draw_inv_dec_array(lib, rows)?;
    let pc = draw_precharge_array(lib, cols)?;
    let read_mux = draw_read_mux_array(lib, cols)?;
    let write_mux = draw_write_mux_array(lib, cols)?;
    let sense_amp = draw_sense_amp_array(lib, cols / 2)?;
    let dffs = draw_dff_array(lib, cols / 2)?;

    let core = Instance {
        cell: core,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "core".to_string(),
        reflect_vert: false,
    };

    let mut nand2_dec = Instance {
        cell: nand2_dec.cell,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "nand2_dec_array".to_string(),
        reflect_vert: false,
    };

    let mut inv_dec = Instance {
        cell: inv_dec.cell,
        inst_name: "inv_dec_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut pc = Instance {
        cell: pc.cell,
        inst_name: "precharge_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut read_mux = Instance {
        cell: read_mux.cell,
        inst_name: "read_mux_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut write_mux = Instance {
        cell: write_mux.cell,
        inst_name: "write_mux_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut sense_amp = Instance {
        cell: sense_amp.cell,
        inst_name: "sense_amp_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut dffs = Instance {
        cell: dffs.cell,
        inst_name: "dff_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    inv_dec.align_to_the_left_of(core.bbox(), 1_000);
    inv_dec.align_centers_vertically_gridded(core.bbox(), grid);

    nand2_dec.align_to_the_left_of(inv_dec.bbox(), grid);
    nand2_dec.align_to_the_left_of(inv_dec.bbox(), 1_000);
    nand2_dec.align_centers_vertically_gridded(core.bbox(), grid);

    pc.align_beneath(core.bbox(), 1_000);
    pc.align_centers_horizontally_gridded(core.bbox(), grid);

    read_mux.align_beneath(pc.bbox(), 1_000);
    read_mux.align_centers_horizontally_gridded(core.bbox(), grid);

    write_mux.align_beneath(read_mux.bbox(), 1_000);
    write_mux.align_centers_horizontally_gridded(core.bbox(), grid);

    sense_amp.align_beneath(write_mux.bbox(), 1_000);
    sense_amp.align_centers_horizontally_gridded(core.bbox(), grid);

    dffs.align_beneath(sense_amp.bbox(), 1_000);
    dffs.align_centers_horizontally_gridded(core.bbox(), grid);

    // Top level routing
    let mut router = Router::new(lib.clone());
    let cfg = router.cfg();

    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    println!("PORTS: {:?}", nand2_dec.ports());
    let port_start = nand2_dec.port("VDD_0").bbox(m0).unwrap();
    let port_stop = nand2_dec
        .port(format!("VDD_{}", rows - 1))
        .bbox(m0)
        .unwrap();

    let vdd_area = Rect::from(port_start.union(&port_stop));
    let trace_hspan = Span::from_center_span_gridded(vdd_area.center().x, cfg.line(1), cfg.grid());

    let trace = router.trace(Rect::from_spans(trace_hspan, vdd_area.vspan()), 1);

    let routing = router.finish();

    layout.insts.push(core);
    layout.insts.push(nand2_dec);
    layout.insts.push(inv_dec);
    layout.insts.push(pc);
    layout.insts.push(read_mux);
    layout.insts.push(write_mux);
    layout.insts.push(sense_amp);
    layout.insts.push(dffs);
    layout.insts.push(routing);

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
