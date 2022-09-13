use layout21::raw::align::AlignRect;
use layout21::raw::geom::Rect;
use layout21::raw::Dir;
use layout21::{
    raw::{BoundBoxTrait, Cell, Instance, Layout, Point, Span},
    utils::Ptr,
};
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::{LayerIdx, PdkLib};

use crate::clog2;
use crate::decoder::DecoderTree;
use crate::layout::decoder::{bus_width, draw_hier_decode, ConnectSubdecodersArgs, GateList};
use crate::layout::dff::draw_vert_dff_array;
use crate::layout::route::grid::{Grid, TrackLocator};
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

    let mut layout = Layout::new(&name);

    assert_eq!(cols % 2, 0);
    assert_eq!(rows % 2, 0);

    let grid = lib.pdk.grid();

    let row_bits = clog2(rows);
    let col_sel_bits = 1;

    let decoder_tree = DecoderTree::new(row_bits);
    assert_eq!(decoder_tree.root.children.len(), 2);

    let decoder1 = draw_hier_decode(lib, "predecoder_1", &decoder_tree.root.children[0])?;
    let decoder2 = draw_hier_decode(lib, "predecoder_2", &decoder_tree.root.children[1])?;
    let addr_dffs = draw_vert_dff_array(lib, "addr_dffs", row_bits + col_sel_bits)?;

    let core = draw_array(rows, cols, lib)?;
    let nand_dec = draw_nand2_array(lib, "nand2_dec", rows)?;
    let inv_dec = draw_inv_dec_array(lib, "inv_dec", rows)?;
    let wldrv_nand = draw_nand2_array(lib, "wldrv_nand2", rows)?;
    let wldrv_inv = draw_inv_dec_array(lib, "wldrv_inv", rows)?;
    let pc = draw_precharge_array(lib, cols)?;
    let read_mux = draw_read_mux_array(lib, cols / 2)?;
    let write_mux = draw_write_mux_array(lib, cols)?;
    let sense_amp = draw_sense_amp_array(lib, cols / 2)?;
    let data_dffs = draw_dff_array(lib, "data_dff_array", cols / 2)?;

    let core = Instance {
        cell: core,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "core".to_string(),
        reflect_vert: false,
    };

    let mut decoder1 = Instance::new("hierarchical_decoder", decoder1);
    let mut decoder2 = Instance::new("hierarchical_decoder", decoder2);

    let mut wldrv_nand = Instance::new("wldrv_nand_array", wldrv_nand.cell);
    let mut wldrv_inv = Instance::new("wldrv_inv_array", wldrv_inv.cell);
    let mut nand_dec = Instance::new("nand2_dec_array", nand_dec.cell);
    let mut inv_dec = Instance::new("inv_dec_array", inv_dec.cell);

    let mut pc = Instance {
        cell: pc,
        inst_name: "precharge_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut read_mux = Instance {
        cell: read_mux,
        inst_name: "read_mux_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut write_mux = Instance {
        cell: write_mux,
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
        cell: data_dffs.cell,
        inst_name: "dff_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut addr_dffs = Instance::new("addr_dffs", addr_dffs);

    let core_bbox = core.bbox();

    wldrv_inv.align_to_the_left_of(core_bbox, 1_000);
    wldrv_inv.align_centers_vertically_gridded(core_bbox, grid);
    wldrv_nand.align_to_the_left_of(wldrv_inv.bbox(), 1_000);
    wldrv_nand.align_centers_vertically_gridded(core_bbox, grid);

    inv_dec.align_to_the_left_of(wldrv_nand.bbox(), 1_000);
    inv_dec.align_centers_vertically_gridded(core_bbox, grid);
    nand_dec.align_to_the_left_of(inv_dec.bbox(), 1_000);
    nand_dec.align_centers_vertically_gridded(core_bbox, grid);

    pc.align_beneath(core_bbox, 1_000);
    pc.align_centers_horizontally_gridded(core_bbox, grid);

    read_mux.align_beneath(pc.bbox(), 1_000);
    read_mux.align_centers_horizontally_gridded(core_bbox, grid);

    write_mux.align_beneath(read_mux.bbox(), 1_000);
    write_mux.align_centers_horizontally_gridded(core_bbox, grid);

    sense_amp.align_beneath(write_mux.bbox(), 1_000);
    sense_amp.align_centers_horizontally_gridded(core_bbox, grid);

    dffs.align_beneath(sense_amp.bbox(), 1_000);
    dffs.align_centers_horizontally_gridded(core_bbox, grid);

    decoder1.align_beneath(core_bbox, 1_000);
    decoder1.align_to_the_left_of(sense_amp.bbox(), 1_000);

    decoder2.align_beneath(decoder1.bbox(), 1_000);
    decoder2.align_to_the_left_of(sense_amp.bbox(), 1_000);

    addr_dffs.align_beneath(decoder2.bbox(), 1_270);
    addr_dffs.align_to_the_left_of(dffs.bbox(), 1_270);

    // Top level routing
    let mut router = Router::new("bank_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    for (nand, inv) in [(&nand_dec, &inv_dec), (&wldrv_nand, &wldrv_inv)] {
        connect(ConnectArgs {
            metal_idx: 1,
            port_idx: 0,
            router: &mut router,
            inst: nand,
            port_name: "VDD",
            count: rows,
            dir: Dir::Vert,
        });
        connect(ConnectArgs {
            metal_idx: 1,
            port_idx: 0,
            router: &mut router,
            inst: nand,
            port_name: "VSS",
            count: rows,
            dir: Dir::Vert,
        });

        connect(ConnectArgs {
            metal_idx: 1,
            port_idx: 0,
            router: &mut router,
            inst: inv,
            port_name: "vdd",
            count: rows,
            dir: Dir::Vert,
        });
        connect(ConnectArgs {
            metal_idx: 1,
            port_idx: 0,
            router: &mut router,
            inst: inv,
            port_name: "gnd",
            count: rows,
            dir: Dir::Vert,
        });
    }

    for i in 0..rows {
        // Connect decoder nand to decoder inverter
        let src = nand_dec.port(format!("Y_{}", i)).largest_rect(m0).unwrap();
        let dst = inv_dec.port(format!("din_{}", i)).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace.s_bend(dst, Dir::Horiz);

        // Connect inverter to WL driver
        let src = inv_dec
            .port(format!("din_b_{}", i))
            .largest_rect(m0)
            .unwrap();
        let dst = wldrv_nand
            .port(format!("A_{}", i))
            .largest_rect(m0)
            .unwrap();
        let mut trace = router.trace(src, 0);
        trace.s_bend(dst, Dir::Horiz);

        // Connect nand wldriver output to inv wldriver input.
        let src = wldrv_nand
            .port(format!("Y_{}", i))
            .largest_rect(m0)
            .unwrap();
        let dst = wldrv_inv
            .port(format!("din_{}", i))
            .largest_rect(m0)
            .unwrap();
        let mut trace = router.trace(src, 0);
        trace.s_bend(dst, Dir::Horiz);

        // Then connect inv decoder output to wordline.
        let src = wldrv_inv
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

    let core_bot = core.bbox().into_rect().bottom();
    let pc_bbox = pc.bbox().into_rect();
    let read_mux_bbox = read_mux.bbox().into_rect();
    let pc_top = pc_bbox.top();
    let pc_midpt = Span::new(pc_top, core_bot).center();

    for i in 0..cols {
        let mut bl_rect = Rect::new(Point::zero(), Point::zero());
        let mut br_rect = Rect::new(Point::zero(), Point::zero());

        for j in 0..2 {
            let bl = if j == 0 { "bl" } else { "br" };
            let src = core.port(format!("bl{j}_{}", i)).largest_rect(m1).unwrap();
            let bl1 = pc.port(format!("{bl}1_{}", i)).largest_rect(m0).unwrap();
            let bl0 = pc.port(format!("{bl}0_{}", i)).largest_rect(m0).unwrap();

            let mut trace = router.trace(src, 1);
            let target = if (i % 2 == 0) ^ (j == 0) {
                bl0.left() - cfg.space(0) - cfg.line(1)
            } else {
                bl0.right() + cfg.space(0) + cfg.line(1)
            };

            trace
                .place_cursor(Dir::Vert, false)
                .vert_to(pc_midpt)
                .horiz_to(target)
                .vert_to(bl0.bottom());

            if j == 0 {
                bl_rect = trace.rect();
            } else {
                br_rect = trace.rect();
            }

            let mut t0 = router.trace(bl0, 0);
            t0.place_cursor_centered().horiz_to_trace(&trace).up();
            let mut t1 = router.trace(bl1, 0);
            t1.place_cursor_centered().horiz_to_trace(&trace).up();
        }

        let vdd_tap_left = pc.port(format!("vdd_{}", i)).largest_rect(m0).unwrap();
        let vdd_tap_right = pc.port(format!("vdd_{}", i + 1)).largest_rect(m0).unwrap();
        let vdd0 = pc
            .port(format!("vdd{}_{}", i % 2, i))
            .largest_rect(m0)
            .unwrap();
        let vdd1 = pc
            .port(format!("vdd{}_{}", 1 - (i % 2), i))
            .largest_rect(m0)
            .unwrap();

        let mut trace = router.trace(vdd0, 0);
        trace.place_cursor_centered().horiz_to(vdd_tap_right.left());

        let mut trace = router.trace(vdd1, 0);
        trace.place_cursor_centered().horiz_to(vdd_tap_left.right());

        let mut trace = router.trace(bl_rect, 1);
        let dst = read_mux
            .port(format!("bl_{}_{}", i % 2, i / 2))
            .largest_rect(m1)
            .unwrap();
        let dst2 = write_mux
            .port(format!("bl_{}", i))
            .largest_rect(m1)
            .unwrap();
        trace
            .place_cursor(Dir::Vert, false)
            .vert_to(pc_bbox.bottom())
            .s_bend(dst, Dir::Vert)
            .vert_to(read_mux_bbox.bottom())
            .s_bend(dst2, Dir::Vert);

        let mut trace = router.trace(br_rect, 1);
        let dst = read_mux
            .port(format!("br_{}_{}", i % 2, i / 2))
            .largest_rect(m1)
            .unwrap();
        let dst2 = write_mux
            .port(format!("br_{}", i))
            .largest_rect(m1)
            .unwrap();
        trace
            .place_cursor(Dir::Vert, false)
            .vert_to(pc_bbox.bottom())
            .s_bend(dst, Dir::Vert)
            .vert_to(read_mux_bbox.bottom())
            .s_bend(dst2, Dir::Vert);

        let src = read_mux
            .port(format!("bl_out_{}", i / 2))
            .largest_rect(m1)
            .unwrap();
        let mut trace = router.trace(src, 1);
    }

    connect(ConnectArgs {
        metal_idx: 2,
        port_idx: 1,
        router: &mut router,
        inst: &pc,
        port_name: "vdd",
        count: cols + 1,
        dir: Dir::Horiz,
    });

    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: None,
            below: Some(ContactPosition::CenteredNonAdjacent),
        },
    );
    let grid = Grid::builder()
        .center(Point::zero())
        .line(cfg.line(1))
        .space(space)
        .grid(lib.pdk.grid())
        .build()?;
    let vspan = Span::new(decoder2.bbox().p0.y, nand_dec.bbox().p1.y);

    let track_start = grid.get_track_index(
        Dir::Vert,
        nand_dec.bbox().into_rect().left(),
        TrackLocator::EndsBefore,
    ) - bus_width(&decoder_tree.root) as isize;
    crate::layout::decoder::connect_subdecoders(ConnectSubdecodersArgs {
        node: &decoder_tree.root,
        grid: &grid,
        track_start,
        vspan,
        router: &mut router,
        gates: GateList::Array(&nand_dec, rows),
        subdecoders: &[&decoder1, &decoder2],
    });

    let routing = router.finish();

    layout.insts.push(core);
    layout.insts.push(decoder1);
    layout.insts.push(decoder2);
    layout.insts.push(wldrv_nand);
    layout.insts.push(wldrv_inv);
    layout.insts.push(nand_dec);
    layout.insts.push(inv_dec);
    layout.insts.push(pc);
    layout.insts.push(read_mux);
    layout.insts.push(write_mux);
    layout.insts.push(sense_amp);
    layout.insts.push(dffs);
    layout.insts.push(addr_dffs);
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
    dir: Dir,
}

fn connect(args: ConnectArgs) {
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
    let trace_xspan = Span::from_center_span_gridded(
        target_area.span(!args.dir).center(),
        3 * cfg.line(args.metal_idx),
        cfg.grid(),
    );

    let dir = args.dir;
    let mut trace = args.router.trace(
        Rect::span_builder()
            .with(dir, target_area.span(dir))
            .with(!dir, trace_xspan)
            .build(),
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

    use crate::utils::{panic_on_err, test_path};

    use super::*;

    #[test]
    fn test_sram_bank_32x32() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_32x32")?;
        draw_sram_bank(32, 32, &mut lib).map_err(panic_on_err)?;

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }

    #[test]
    fn test_sram_bank_16x16() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_16x16")?;
        draw_sram_bank(16, 16, &mut lib).map_err(panic_on_err)?;

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }

    #[test]
    fn test_sram_bank_128x128() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_128x128")?;
        draw_sram_bank(128, 128, &mut lib).map_err(panic_on_err)?;

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }
}
