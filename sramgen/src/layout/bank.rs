use layout21::raw::align::AlignRect;
use layout21::raw::geom::Rect;
use layout21::raw::Dir;
use layout21::{
    raw::{BoundBoxTrait, Cell, Instance, Layout, Point, Span},
    utils::Ptr,
};
use pdkprims::{LayerIdx, PdkLib};

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
    let m2 = cfg.layerkey(2);

    vertical_connect(ConnectArgs {
        metal_idx: 1,
        port_idx: 0,
        router: &mut router,
        inst: &nand2_dec,
        port_name: "VDD",
        count: rows,
    });
    vertical_connect(ConnectArgs {
        metal_idx: 1,
        port_idx: 0,
        router: &mut router,
        inst: &nand2_dec,
        port_name: "VSS",
        count: rows,
    });

    vertical_connect(ConnectArgs {
        metal_idx: 1,
        port_idx: 0,
        router: &mut router,
        inst: &inv_dec,
        port_name: "vdd",
        count: rows,
    });
    vertical_connect(ConnectArgs {
        metal_idx: 1,
        port_idx: 0,
        router: &mut router,
        inst: &inv_dec,
        port_name: "gnd",
        count: rows,
    });

    for i in 0..rows {
        // Connect nand decoder output to inv decoder input.
        let src = nand2_dec.port(format!("Y_{}", i)).largest_rect(m0).unwrap();
        let dst = inv_dec.port(format!("din_{}", i)).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace.s_bend(dst, Dir::Horiz);

        // Then connect inv decoder output to wordline.
        let src = inv_dec
            .port(format!("din_b_{}", i))
            .largest_rect(m0)
            .unwrap();
        let dst = core.port(format!("wl_{}", i)).largest_rect(m2).unwrap();
        let mut trace = router.trace(src, 0);
        // move right
        trace
            .place_cursor(Dir::Horiz, true)
            .up()
            .up()
            .set_min_width()
            .s_bend(dst, Dir::Horiz);
    }

    for i in 0..cols {
        let src = core.port(format!("bl0_{}", i)).largest_rect(m1).unwrap();
        let mut trace = router.trace(src, 1);
    }

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

struct ConnectArgs<'a> {
    metal_idx: LayerIdx,
    port_idx: LayerIdx,
    router: &'a mut Router,
    inst: &'a Instance,
    port_name: &'a str,
    count: usize,
}

fn vertical_connect(args: ConnectArgs) {
    let cfg = args.router.cfg();
    let m0 = cfg.layerkey(args.port_idx);
    let port_start = args
        .inst
        .port(format!("{}_0", args.port_name))
        .bbox(m0)
        .unwrap();
    let port_stop = args
        .inst
        .port(format!("{}_{}", args.port_name, args.count - 1))
        .bbox(m0)
        .unwrap();

    let target_area = Rect::from(port_start.union(&port_stop));
    let trace_hspan = Span::from_center_span_gridded(
        target_area.center().x,
        3 * cfg.line(args.metal_idx),
        cfg.grid(),
    );
    let mut trace = args.router.trace(
        Rect::from_spans(trace_hspan, target_area.vspan()),
        args.metal_idx,
    );

    for i in 0..args.count {
        let port = args
            .inst
            .port(format!("{}_{}", args.port_name, i))
            .bbox(m0)
            .unwrap();
        trace.contact_down(port.into());
    }
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
