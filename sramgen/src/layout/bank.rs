use derive_builder::Builder;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Rect;
use layout21::raw::translate::Translate;
use layout21::raw::{AbstractPort, BoundBoxTrait, Cell, Dir, Instance, Int, Layout, Point, Span};
use layout21::utils::Ptr;
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::{LayerIdx, PdkLib};

use crate::clog2;
use crate::decoder::DecoderTree;
use crate::layout::col_inv::draw_col_inv_array;
use crate::layout::decoder::{
    bus_width, draw_hier_decode, ConnectSubdecodersArgs, GateArrayParams,
};
use crate::layout::dff::{draw_dff_grid, draw_vert_dff_array, DffGridParams};
use crate::layout::guard_ring::{draw_guard_ring, GuardRingParams};
use crate::layout::power::{PowerStrapGen, PowerStrapOpts};
use crate::layout::route::grid::{Grid, TrackLocator};
use crate::layout::route::Router;
use crate::layout::tmc::{draw_tmc, TmcParams};
use crate::precharge::{PrechargeArrayParams, PrechargeParams};
use crate::tech::{BITCELL_HEIGHT, COLUMN_WIDTH};

use super::array::draw_array;
use super::decoder::{draw_inv_dec_array, draw_nand2_dec_array};
use super::mux::{draw_read_mux_array, draw_write_mux_array};
use super::precharge::draw_precharge_array;
use super::route::Trace;
use super::sense_amp::draw_sense_amp_array;
use super::Result;

pub const M1_PWR_OVERHANG: Int = 200;

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
    let decoder1_bits = clog2(decoder_tree.root.children[0].num);
    let addr_dffs = draw_vert_dff_array(lib, "addr_dffs", row_bits + col_sel_bits)?;

    let core = draw_array(rows, cols, lib)?;
    let nand_dec = draw_nand2_dec_array(
        lib,
        GateArrayParams {
            prefix: "nand2_dec",
            width: rows,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;
    let inv_dec = draw_inv_dec_array(
        lib,
        GateArrayParams {
            prefix: "inv_dec",
            width: rows,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;
    let wldrv_nand = draw_nand2_dec_array(
        lib,
        GateArrayParams {
            prefix: "wldrv_nand",
            width: rows,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;
    let wldrv_inv = draw_inv_dec_array(
        lib,
        GateArrayParams {
            prefix: "wldrv_inv",
            width: rows,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;
    let pc = draw_precharge_array(
        lib,
        PrechargeArrayParams {
            width: cols,
            instance_params: PrechargeParams {
                name: "precharge".to_string(),
                length: 150,
                pull_up_width: 1_200,
                equalizer_width: 1_000,
            },
            name: "precharge_array".to_string(),
        },
    )?;
    let read_mux = draw_read_mux_array(lib, cols / 2, 2)?;
    let write_mux = draw_write_mux_array(lib, cols, 2, 1)?;
    let col_inv = draw_col_inv_array(lib, "col_data_inv", cols / 2)?;
    let sense_amp = draw_sense_amp_array(lib, cols / 2)?;
    let din_dff_params = DffGridParams::builder()
        .name("data_dff_array")
        .rows(2)
        .cols(cols / 4)
        .row_pitch(4 * COLUMN_WIDTH)
        .build()?;
    let din_dffs = draw_dff_grid(lib, din_dff_params)?;
    let tmc = draw_tmc(
        lib,
        TmcParams {
            name: "tmc".to_string(),
            multiplier: 6,
            units: 16,
        },
    )?;

    let core = Instance {
        cell: core,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "core".to_string(),
        reflect_vert: false,
    };

    let mut decoder1 = Instance::new("hierarchical_decoder", decoder1);
    let mut decoder2 = Instance::new("hierarchical_decoder", decoder2);
    let mut wldrv_nand = Instance::new("wldrv_nand_array", wldrv_nand);
    let mut wldrv_inv = Instance::new("wldrv_inv_array", wldrv_inv);
    let mut nand_dec = Instance::new("nand2_dec_array", nand_dec);
    let mut inv_dec = Instance::new("inv_dec_array", inv_dec);
    let mut pc = Instance::new("precharge_array", pc);
    let mut read_mux = Instance::new("read_mux_array", read_mux);
    let mut write_mux = Instance::new("write_mux_array", write_mux);
    let mut col_inv = Instance::new("col_inv_array", col_inv);
    let mut sense_amp = Instance::new("sense_amp_array", sense_amp);
    let mut din_dffs = Instance::new("dff_array", din_dffs);
    let mut addr_dffs = Instance::new("addr_dffs", addr_dffs);
    let mut tmc = Instance::new("tmc", tmc);

    let core_bbox = core.bbox();

    wldrv_inv.align_to_the_left_of(core_bbox, 1_270);
    wldrv_inv.align_centers_vertically_gridded(core_bbox, grid);
    wldrv_nand.align_to_the_left_of(wldrv_inv.bbox(), 1_000);
    wldrv_nand.align_centers_vertically_gridded(core_bbox, grid);

    inv_dec.align_to_the_left_of(wldrv_nand.bbox(), 1_000);
    inv_dec.align_centers_vertically_gridded(core_bbox, grid);
    nand_dec.align_to_the_left_of(inv_dec.bbox(), 1_000);
    nand_dec.align_centers_vertically_gridded(core_bbox, grid);

    pc.align_beneath(core_bbox, 1_270);
    pc.align_centers_horizontally_gridded(core_bbox, grid);

    read_mux.align_beneath(pc.bbox(), 1_000);
    read_mux.align_centers_horizontally_gridded(core_bbox, grid);

    write_mux.align_beneath(read_mux.bbox(), 1_000);
    write_mux.align_centers_horizontally_gridded(core_bbox, grid);

    col_inv.align_beneath(write_mux.bbox(), 1_000);
    col_inv.align_centers_horizontally_gridded(core_bbox, grid);

    sense_amp.align_beneath(col_inv.bbox(), 1_000);
    sense_amp.align_centers_horizontally_gridded(core_bbox, grid);

    din_dffs.align_beneath(sense_amp.bbox(), 1_000);
    din_dffs.align_centers_horizontally_gridded(core_bbox, grid);

    decoder1.align_beneath(core_bbox, 1_000);
    decoder1.align_to_the_left_of(sense_amp.bbox(), 1_000);

    decoder2.align_beneath(decoder1.bbox(), 1_270);
    decoder2.align_to_the_left_of(sense_amp.bbox(), 1_000);

    addr_dffs.align_top(decoder2.bbox());

    tmc.align_above(din_dffs.bbox(), 1_270);
    tmc.align_to_the_right_of(core_bbox, 1_270);

    // Top level routing
    let mut router = Router::new("bank_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    let mut power_grid = PowerStrapGen::new(
        PowerStrapOpts::builder()
            .h_metal(2)
            .h_line(5 * cfg.line(2))
            .h_space(8 * cfg.line(2))
            .v_metal(3)
            .v_line(400)
            .v_space(400)
            .pdk(lib.pdk.clone())
            .name("bank_power_strap")
            .enclosure(Rect::new(Point::zero(), Point::zero()))
            .build()?,
    );

    for i in 0..rows {
        // Connect decoder nand to decoder inverter
        let src = nand_dec.port(format!("y_{}", i)).largest_rect(m0).unwrap();
        let dst = inv_dec.port(format!("din_{}", i)).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace.s_bend(dst, Dir::Horiz);

        // Connect inverter to WL driver
        let src = inv_dec
            .port(format!("din_b_{}", i))
            .largest_rect(m0)
            .unwrap();
        let dst = wldrv_nand
            .port(format!("a_{}", i))
            .largest_rect(m0)
            .unwrap();
        let mut trace = router.trace(src, 0);
        trace.s_bend(dst, Dir::Horiz);

        // Connect nand wldriver output to inv wldriver input.
        let src = wldrv_nand
            .port(format!("y_{}", i))
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
        let m2_block = src.bbox().union(&dst.bbox()).into_rect();
        power_grid.add_padded_blockage(2, m2_block);
    }

    power_grid.add_padded_blockage(2, core_bbox.into_rect());
    let core_bot = core_bbox.into_rect().bottom();
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
    }

    let sa_bbox = sense_amp.bbox().into_rect();
    let bl_bot = sense_amp.port("inn_0").largest_rect(m2).unwrap().bottom();
    // Route read bitlines
    for i in 0..cols / 2 {
        // Route data and data bar to 2:1 write muxes
        let data_b_pin = write_mux
            .port(format!("data_b_{}", i))
            .largest_rect(m2)
            .unwrap();
        let data_pin = write_mux
            .port(format!("data_{}", i))
            .largest_rect(m2)
            .unwrap();

        let src = col_inv
            .port(format!("din_b_{}", i))
            .largest_rect(m0)
            .unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Vert, true)
            .up()
            .vert_to(data_b_pin.top())
            .contact_up(data_b_pin);

        let bl = read_mux
            .port(format!("bl_out_{}", i))
            .largest_rect(m2)
            .unwrap();
        let br = read_mux
            .port(format!("br_out_{}", i))
            .largest_rect(m2)
            .unwrap();
        let center = Span::merge([bl.hspan(), br.hspan()]).center();

        let data_span = Span::from_center_span_gridded(center, cfg.line(3), cfg.grid());
        let bl_span = Span::new(
            data_span.start() - 2 * cfg.space(3) - cfg.line(3),
            data_span.start() - 2 * cfg.space(3),
        );
        let br_span = Span::new(
            data_span.stop() + 2 * cfg.space(3),
            data_span.stop() + 2 * cfg.space(3) + cfg.line(3),
        );

        let bl_vspan = Span::new(bl_bot, bl.bottom());

        let mut bl_m3 = router.trace(Rect::from_spans(bl_span, bl_vspan), 3);
        let mut br_m3 = router.trace(Rect::from_spans(br_span, bl_vspan), 3);
        power_grid.add_padded_blockage(3, bl_m3.rect());
        power_grid.add_padded_blockage(3, br_m3.rect());

        let inp = sense_amp
            .port(format!("inp_{}", i))
            .largest_rect(m2)
            .unwrap();
        let inn = sense_amp
            .port(format!("inn_{}", i))
            .largest_rect(m2)
            .unwrap();

        for (src, m3, dst) in [(bl, &mut bl_m3, inp), (br, &mut br_m3, inn)] {
            router
                .trace(src, 2)
                .place_cursor(Dir::Vert, false)
                .set_width(cfg.line(2))
                .horiz_to_trace(m3)
                .contact_up(m3.rect());
            m3.contact_down(dst);
        }

        // data
        let data_rect = Rect::from_spans(data_span, sa_bbox.vspan());
        power_grid.add_padded_blockage(3, data_rect);
        let mut trace = router.trace(data_rect, 3);
        let dst1 = col_inv.port(format!("din_{}", i)).largest_rect(m0).unwrap();
        trace.place_cursor(Dir::Vert, true).vert_to(dst1.top());
        power_grid.add_padded_blockage(3, trace.rect());
        trace.down().down().down();

        // Route din dff to data_rect
        let src = din_dffs.port(format!("q_{}", i)).largest_rect(m2).unwrap();
        let mut trace = router.trace(src, 2);
        let voffset = if i % 2 == 0 { 1_200 } else { -800 };
        trace
            .place_cursor_centered()
            .down()
            .vert_to(src.center().y + voffset)
            .up()
            .set_width(cfg.line(3))
            .horiz_to_rect(data_rect)
            .up()
            .set_min_width()
            .vert_to_rect(data_rect);

        let mut trace = router.trace(data_rect, 3);
        trace
            .place_cursor(Dir::Vert, true)
            .vert_to(data_pin.top())
            .contact_down(data_pin);
    }

    let trace = connect(ConnectArgs {
        metal_idx: 2,
        port_idx: 1,
        router: &mut router,
        insts: GateList::Array(&pc, cols + 1),
        port_name: "vdd",
        dir: Dir::Horiz,
        overhang: Some(100),
        transverse_offset: 0,
    });
    power_grid.add_vdd_target(2, trace.rect());

    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: Some(ContactPosition::CenteredNonAdjacent),
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

    let bus_width = bus_width(&decoder_tree.root);

    let track_start = grid.get_track_index(
        Dir::Vert,
        nand_dec.bbox().into_rect().left(),
        TrackLocator::EndsBefore,
    ) - bus_width as isize;
    crate::layout::decoder::connect_subdecoders(ConnectSubdecodersArgs {
        node: &decoder_tree.root,
        grid: &grid,
        track_start,
        vspan,
        router: &mut router,
        gates: GateList::Array(&nand_dec, rows),
        subdecoders: &[&decoder1, &decoder2],
    });

    let bbox = router.cell().bbox();
    addr_dffs.align_to_the_left_of(bbox, 1_270);

    let track_start = track_start + bus_width as isize;
    let traces = (track_start..(track_start + 2 * row_bits as isize))
        .map(|track| {
            let rect = Rect::span_builder()
                .with(Dir::Vert, Span::new(addr_dffs.bbox().p0.y, core_bbox.p0.y))
                .with(Dir::Horiz, grid.vtrack(track))
                .build();
            router.trace(rect, 1)
        })
        .collect::<Vec<_>>();

    for i in 0..row_bits {
        for (port, addr_prefix, idx) in [("q", "addr", 2 * i), ("qn", "addr_b", 2 * i + 1)] {
            let src = addr_dffs
                .port(format!("{}_{}", port, i + col_sel_bits))
                .largest_rect(m2)
                .unwrap();
            let mut trace = router.trace(src, 2);
            trace
                .place_cursor_centered()
                .horiz_to_trace(&traces[idx])
                .contact_down(traces[idx].rect());
            power_grid.add_padded_blockage(2, trace.rect().expand(cfg.space(2)));

            let (target_port, target_idx) = if i < decoder1_bits {
                // Route to decoder1
                (decoder1.port(format!("{}_{}", addr_prefix, i)), i)
            } else {
                // Route to decoder2
                (
                    decoder2.port(format!("{}_{}", addr_prefix, i - decoder1_bits)),
                    i - decoder1_bits,
                )
            };
            let mut target = target_port.largest_rect(m1).unwrap();
            let base = target.p0.y + 160 + 600 * (2 * target_idx + idx % 2) as isize;
            let top = base + 320;
            assert!(top <= target.p1.y);
            target.p0.y = base;
            target.p1.y = top;
            let mut trace = router.trace(target, 1);
            trace
                .place_cursor_centered()
                .up()
                .horiz_to_trace(&traces[idx])
                .contact_down(traces[idx].rect());
            power_grid.add_padded_blockage(2, trace.rect().expand(cfg.space(2)));
        }
    }

    let sense_amp_bbox = sense_amp.bbox().into_rect();
    let column_blockage = Rect::from_spans(
        sense_amp_bbox.hspan(),
        Span::new(sense_amp_bbox.bottom(), pc_bbox.top()),
    );
    power_grid.add_padded_blockage(2, column_blockage);

    // power strapping - metal 1
    for instance in [
        &decoder1,
        &decoder2,
        &wldrv_nand,
        &wldrv_inv,
        &nand_dec,
        &inv_dec,
    ] {
        for name in ["vpb", "vdd"] {
            for port in instance.ports_starting_with(name) {
                power_grid.add_vdd_target(1, port.largest_rect(m1).unwrap());
            }
        }
        for name in ["vnb", "vss"] {
            for port in instance.ports_starting_with(name) {
                power_grid.add_gnd_target(1, port.largest_rect(m1).unwrap());
            }
        }
    }
    // power strapping - metal 2
    for instance in [&read_mux, &write_mux, &col_inv, &sense_amp] {
        for name in ["vpb", "vdd"] {
            for port in instance.ports_starting_with(name) {
                power_grid.add_vdd_target(2, port.largest_rect(m2).unwrap());
            }
        }
        for name in ["vnb", "vss"] {
            for port in instance.ports_starting_with(name) {
                power_grid.add_gnd_target(2, port.largest_rect(m2).unwrap());
            }
        }
    }

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
    layout.insts.push(col_inv);
    layout.insts.push(sense_amp);
    layout.insts.push(din_dffs.clone());
    layout.insts.push(addr_dffs);
    // layout.insts.push(tmc);

    let bbox = layout.bbox();

    power_grid.set_enclosure(bbox);
    power_grid.add_blockage(2, core_bbox.into_rect());

    layout.insts.push(power_grid.generate()?);

    let guard_ring = draw_guard_ring(
        lib,
        GuardRingParams {
            enclosure: bbox.into_rect().expand(3_000),
            prefix: "sram_guard_ring".to_string(),
        },
    )?;
    let guard_ring = Instance::new("sram_guard_ring", guard_ring);
    let guard_ring_bbox = guard_ring.bbox().into_rect();

    for i in 0..(cols / 2) {
        let src = din_dffs.port(format!("d_{i}")).largest_rect(m2).unwrap();
        let offset = if i % 2 == 0 {
            -3 * cfg.line(3)
        } else {
            3 * cfg.line(3)
        };
        let mut trace = router.trace(src, 2);
        let cx = src.center().x;
        trace
            .place_cursor_centered()
            .horiz_to(cx + offset)
            .up()
            .set_min_width()
            .vert_to(guard_ring_bbox.bottom());
    }

    layout.insts.push(guard_ring);

    let routing = router.finish();
    layout.insts.push(routing);

    // Draw dnwell
    let dnwell_rect = bbox.into_rect().expand(1_600);
    layout.draw_rect(lib.pdk.get_layerkey("dnwell").unwrap(), dnwell_rect);

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[derive(Builder)]
#[builder(pattern = "owned")]
pub(crate) struct ConnectArgs<'a> {
    pub(crate) metal_idx: LayerIdx,
    pub(crate) port_idx: LayerIdx,
    pub(crate) router: &'a mut Router,
    pub(crate) insts: GateList<'a>,
    pub(crate) port_name: &'a str,
    pub(crate) dir: Dir,
    #[builder(setter(strip_option), default)]
    pub(crate) overhang: Option<isize>,
    #[builder(default)]
    pub(crate) transverse_offset: isize,
}

impl<'a> ConnectArgs<'a> {
    #[inline]
    pub fn builder() -> ConnectArgsBuilder<'a> {
        ConnectArgsBuilder::default()
    }
}

#[derive(Copy, Clone)]
pub(crate) enum GateList<'a> {
    Cells(&'a [Instance]),
    Array(&'a Instance, usize),
    ArraySlice(&'a Instance, usize, usize),
}

impl<'a> GateList<'a> {
    #[inline]
    pub(crate) fn width(&self) -> usize {
        match self {
            Self::Cells(v) => v.len(),
            Self::Array(_, width) => *width,
            Self::ArraySlice(_, _, width) => *width,
        }
    }

    pub(crate) fn port(&self, name: &str, num: usize) -> AbstractPort {
        match self {
            Self::Cells(v) => v[num].port(name),
            Self::Array(v, _) => v.port(format!("{}_{}", name, num)),
            Self::ArraySlice(v, start, _) => v.port(format!("{}_{}", name, start + num)),
        }
    }
}

pub(crate) fn connect(args: ConnectArgs) -> Trace {
    let cfg = args.router.cfg();
    let m0 = cfg.layerkey(args.port_idx);
    let port_start = args.insts.port(args.port_name, 0).bbox(m0).unwrap();
    let port_stop = args
        .insts
        .port(args.port_name, args.insts.width() - 1)
        .bbox(m0)
        .unwrap();

    let target_area = Rect::from(port_start.union(&port_stop));
    let mut span = target_area.span(args.dir);
    let trace_xspan = Span::from_center_span_gridded(
        target_area.span(!args.dir).center(),
        3 * cfg.line(args.metal_idx),
        cfg.grid(),
    );

    if let Some(overhang) = args.overhang {
        span.expand(true, overhang).expand(false, overhang);
    }

    let mut rect = Rect::span_builder()
        .with(args.dir, span)
        .with(!args.dir, trace_xspan)
        .build();

    rect.translate(match args.dir {
        Dir::Horiz => Point::new(0, args.transverse_offset),
        Dir::Vert => Point::new(args.transverse_offset, 0),
    });

    let mut trace = args.router.trace(rect, args.metal_idx);

    for i in 0..args.insts.width() {
        let port = args.insts.port(args.port_name, i).bbox(m0).unwrap();
        trace.contact_down(port.into());
    }

    trace
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
    fn test_sram_bank_128x64() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_128x64")?;
        draw_sram_bank(128, 64, &mut lib).map_err(panic_on_err)?;

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
}
