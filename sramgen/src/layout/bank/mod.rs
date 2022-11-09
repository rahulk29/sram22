use derive_builder::Builder;
use layout21::lef21::LefLibrary;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Rect;
use layout21::raw::translate::Translate;
use layout21::raw::{AbstractPort, BoundBox, BoundBoxTrait, Cell, Dir, Instance, Int, Point, Span};
use layout21::utils::Ptr;
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::{LayerIdx, PdkLib};

use crate::clog2;
use crate::config::ControlMode;
use crate::decoder::DecoderTree;
use crate::gate::{AndParams, Size};
use crate::layout::array::draw_power_connector;
use crate::layout::col_inv::draw_col_inv_array;
use crate::layout::control::draw_control_logic;
use crate::layout::decoder::{
    bus_width, draw_hier_decode, ConnectSubdecodersArgs, GateArrayParams,
};
use crate::layout::dff::{draw_dff_grid, DffGridParams};
use crate::layout::guard_ring::{draw_guard_ring, GuardRingParams};
use crate::layout::power::{PowerSource, PowerStrapGen, PowerStrapOpts};
use crate::layout::route::grid::{Grid, TrackLocator};
use crate::layout::route::Router;
use crate::layout::tmc::{draw_tmc, TmcParams};
use crate::layout::wmask_control::draw_write_mask_control;
use crate::precharge::{PrechargeArrayParams, PrechargeParams};
use crate::tech::{BITCELL_HEIGHT, COLUMN_WIDTH};
use crate::wmask_control::WriteMaskControlParams;

use super::array::draw_array;
use super::decoder::{draw_inv_dec_array, draw_nand2_dec_array};
use super::mux::read::draw_read_mux_array;
use super::mux::write::draw_write_mux_array;
use super::precharge::draw_precharge_array;
use super::route::Trace;
use super::sense_amp::draw_sense_amp_array;
use super::Result;

pub mod lef;

pub const M1_PWR_OVERHANG: Int = 200;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum Side {
    Left,
    Right,
    Bottom,
    Top,
}

impl Side {
    #[inline]
    pub fn dir(&self) -> Dir {
        match *self {
            Side::Left | Side::Right => Dir::Horiz,
            Side::Bottom | Side::Top => Dir::Vert,
        }
    }

    /// Indicates if this side is a positive-going direction
    pub fn pos(&self) -> bool {
        match *self {
            Side::Left | Side::Bottom => false,
            Side::Right | Side::Top => true,
        }
    }
}

pub struct PhysicalDesign {
    pub cell: Ptr<Cell>,
    pub lef: LefLibrary,
}

pub struct SramBankParams {
    rows: usize,
    cols: usize,
    mux_ratio: usize,
    wmask_groups: usize,
}

pub fn draw_sram_bank(lib: &mut PdkLib, params: SramBankParams) -> Result<PhysicalDesign> {
    let SramBankParams {
        rows,
        cols,
        mux_ratio,
        wmask_groups,
    } = params;

    let name = "sram_bank".to_string();
    let mut cell = Cell::empty(&name);

    ////////////////////////////////////////////////////////////////////
    // Validate parameters
    ////////////////////////////////////////////////////////////////////
    assert_eq!(cols % 2, 0);
    assert_eq!(rows % 2, 0);
    assert!(mux_ratio >= 2);
    assert!(wmask_groups >= 1);
    assert_eq!(cols % (mux_ratio * wmask_groups), 0);

    let grid = lib.pdk.grid();

    let row_bits = clog2(rows);
    let col_sel_bits = clog2(mux_ratio);
    let total_addr_bits = row_bits + col_sel_bits;

    ////////////////////////////////////////////////////////////////////
    // Generate subcells
    ////////////////////////////////////////////////////////////////////
    let decoder_tree = DecoderTree::new(row_bits);
    assert_eq!(decoder_tree.root.children.len(), 2);

    let col_decoder = if mux_ratio > 2 {
        let col_decoder_tree = DecoderTree::new(clog2(mux_ratio));
        assert_eq!(
            col_decoder_tree.root.children.len(),
            0,
            "Only 1-level column decoders are supported"
        );
        let col_decoder = draw_hier_decode(lib, "col_decoder", &col_decoder_tree.root)?;
        Some(col_decoder)
    } else {
        None
    };

    let control = draw_control_logic(lib, ControlMode::Simple)?;
    let we_control = draw_write_mask_control(
        lib,
        WriteMaskControlParams {
            name: "write_mask_control".to_string(),
            width: mux_ratio as i64,
            and_params: AndParams {
                name: "write_mask_control_and2".to_string(),
                nand_size: Size {
                    nmos_width: 1_200,
                    pmos_width: 1_800,
                },
                inv_size: Size {
                    nmos_width: 1_200,
                    pmos_width: 1_800,
                },
                length: 150,
            },
        },
    )?;
    let decoder1 = draw_hier_decode(lib, "predecoder_1", &decoder_tree.root.children[0])?;
    let decoder2 = draw_hier_decode(lib, "predecoder_2", &decoder_tree.root.children[1])?;
    let decoder1_bits = clog2(decoder_tree.root.children[0].num);
    let decoder2_bits = clog2(decoder_tree.root.children[1].num);
    let addr_dff_params = DffGridParams::builder()
        .name("addr_dff_array")
        .rows(total_addr_bits + 1) // 1 extra bit for write enable
        .cols(1)
        .row_pitch(COLUMN_WIDTH)
        .build()?;
    let addr_dffs = draw_dff_grid(lib, addr_dff_params)?;

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
                pull_up_width: 1_000,
                equalizer_width: 1_000,
            },
            name: "precharge_array".to_string(),
        },
    )?;
    let read_mux = draw_read_mux_array(lib, cols, mux_ratio)?;
    let write_mux = draw_write_mux_array(lib, cols, mux_ratio, wmask_groups)?;
    let col_inv = draw_col_inv_array(lib, "col_data_inv", cols / mux_ratio, mux_ratio)?;
    let sense_amp = draw_sense_amp_array(lib, cols / mux_ratio, COLUMN_WIDTH * mux_ratio as isize)?;
    let din_dff_params = DffGridParams::builder()
        .name("data_dff_array")
        .rows(2)
        .cols(cols / (2 * mux_ratio))
        .row_pitch(2 * mux_ratio as isize * COLUMN_WIDTH)
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

    let mut router = Router::new("bank_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);
    let m3 = cfg.layerkey(3);

    ////////////////////////////////////////////////////////////////////
    // Create instances
    ////////////////////////////////////////////////////////////////////
    let core = Instance::new("core", core);
    let core_pwr = draw_power_connector(lib, &core)?;
    let core_pwr = Instance::new("core_power", core_pwr);
    let mut control = Instance::new("control_logic", control);
    control.angle = Some(90f64);
    let mut we_control = Instance::new("write_mask_control", we_control);
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
    let mut col_decoder = col_decoder.map(|decoder| Instance::new("col_decoder", decoder));
    let mut tmc = Instance::new("tmc", tmc);

    ////////////////////////////////////////////////////////////////////
    // Place (most) instances
    ////////////////////////////////////////////////////////////////////
    let col_bus_space = (2 * mux_ratio as isize + 3)
        * (std::cmp::max(cfg.line(1), cfg.line(0)) + cfg.space(1))
        + 1_270;
    let core_bbox = core.bbox();

    wldrv_inv.align_to_the_left_of(core_bbox, std::cmp::max(col_bus_space, 7_000));
    wldrv_inv.align_centers_vertically_gridded(core_bbox, grid);
    wldrv_nand.align_to_the_left_of(wldrv_inv.bbox(), 1_000);
    wldrv_nand.align_centers_vertically_gridded(core_bbox, grid);
    let wldrv_nand_bbox = wldrv_nand.bbox().into_rect();

    inv_dec.align_to_the_left_of(wldrv_nand.bbox(), 1_000);
    inv_dec.align_centers_vertically_gridded(core_bbox, grid);
    nand_dec.align_to_the_left_of(inv_dec.bbox(), 1_000);
    nand_dec.align_centers_vertically_gridded(core_bbox, grid);

    pc.align_beneath(core_bbox, 2_900);
    pc.align_centers_horizontally_gridded(core_bbox, grid);

    read_mux.align_beneath(pc.bbox(), 1_000);
    read_mux.align_centers_horizontally_gridded(core_bbox, grid);

    write_mux.align_beneath(read_mux.bbox(), 1_000);
    write_mux.align_centers_horizontally_gridded(core_bbox, grid);

    col_inv.align_beneath(write_mux.bbox(), 1_000);
    col_inv.align_centers_horizontally_gridded(core_bbox, grid);

    sense_amp.align_beneath(col_inv.bbox(), 2_900);
    sense_amp.align_centers_horizontally_gridded(core_bbox, grid);
    sense_amp.reflect_vert_anchored();

    let sa_bbox = sense_amp.bbox().into_rect();
    let pc_bbox = pc.bbox().into_rect();
    let read_mux_bbox = read_mux.bbox().into_rect();
    let write_mux_bbox = write_mux.bbox().into_rect();
    let col_inv_bbox = col_inv.bbox().into_rect();

    let mut col_bbox = BoundBox::empty();
    for bbox in [
        sa_bbox,
        pc_bbox,
        col_inv_bbox,
        read_mux_bbox,
        write_mux_bbox,
    ] {
        col_bbox = col_bbox.union(&bbox.bbox());
    }

    din_dffs.align_beneath(sense_amp.bbox(), 1_000);
    din_dffs.align_centers_horizontally_gridded(core_bbox, grid);

    decoder1.align_beneath(core_bbox, 1_000);
    decoder1.align_to_the_left_of(col_bbox.bbox(), col_bus_space);

    let decoder1_bbox = decoder1.bbox();
    decoder2.align_beneath(decoder1_bbox, 1_270);
    decoder2.align_to_the_left_of(col_bbox.bbox(), col_bus_space);

    let decoder2_bbox = decoder2.bbox();

    let col_dec_bounds = BoundBox {
        p0: Point::new(
            decoder2_bbox.p0.x,
            std::cmp::min(decoder2_bbox.p0.y, write_mux_bbox.bottom()),
        ),
        p1: decoder2_bbox.p1,
    };
    we_control.align_beneath(col_dec_bounds, 1_270);
    we_control.align_to_the_left_of(col_bbox.bbox(), col_bus_space);
    let we_control_bbox = we_control.bbox();

    addr_dffs.align_beneath(decoder1_bbox, 2_000);

    let bbox = if let Some(ref mut col_decoder) = col_decoder {
        col_decoder.align_to_the_left_of(we_control_bbox, 1_270);
        col_decoder.align_centers_vertically_gridded(we_control_bbox, lib.pdk.grid());
        col_decoder.reflect_horiz_anchored();
        col_decoder.bbox()
    } else {
        we_control_bbox
    };

    control.align_beneath(bbox, 1_270);
    control.align_left(decoder2_bbox);
    let control_bbox = control.bbox().into_rect();

    tmc.align_above(din_dffs.bbox(), 1_270);
    tmc.align_to_the_right_of(core_bbox, 1_270);

    let mut power_grid = PowerStrapGen::new(
        PowerStrapOpts::builder()
            .h_metal(2)
            .h_line(640)
            .h_space(3 * cfg.space(2))
            .v_metal(3)
            .v_line(400)
            .v_space(400)
            .pdk(lib.pdk.clone())
            .name("bank_power_strap")
            .enclosure(Rect::new(Point::zero(), Point::zero()))
            .build()?,
    );

    power_grid.add_padded_blockage(
        2,
        control
            .port("clk")
            .largest_rect(m2)
            .unwrap()
            .expand(cfg.line(2) / 2),
    );

    ////////////////////////////////////////////////////////////////////
    // Row routing
    ////////////////////////////////////////////////////////////////////
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

        // Then connect inv decoder output to wordline
        let src = wldrv_inv
            .port(format!("din_b_{}", i))
            .largest_rect(m0)
            .unwrap();
        let dst = core.port(format!("wl_{}", i)).largest_rect(m2).unwrap();
        let mut trace = router.trace(src, 0);
        trace.place_cursor(Dir::Horiz, true).set_min_width();
        let contact_block = trace.cursor_rect().expand(70);
        trace
            .up()
            .up()
            .set_width(170)
            .vert_to_rect(dst)
            .horiz_to_rect(dst);
        let m2_block = trace
            .rect()
            .bbox()
            .union(&dst.bbox())
            .into_rect()
            .expand(75);
        power_grid.add_padded_blockage(2, m2_block);
        power_grid.add_padded_blockage(2, contact_block);
    }

    ////////////////////////////////////////////////////////////////////
    // Column routing
    ////////////////////////////////////////////////////////////////////
    let core_bot = core_bbox.into_rect().bottom();
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

    let bl_bot = sense_amp.port("inp_0").largest_rect(m2).unwrap().bottom();

    let mut dout_spans = Vec::with_capacity(cols / mux_ratio);
    let mut dout_b_spans = Vec::with_capacity(cols / mux_ratio);
    // Route read bitlines
    for i in 0..(cols / mux_ratio) {
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

        if mux_ratio == 2 {
            trace
                .place_cursor(Dir::Vert, true)
                .up()
                .vert_to(data_b_pin.top())
                .contact_up(data_b_pin);
        } else {
            trace
                .place_cursor(Dir::Vert, true)
                .up()
                .up()
                .left_by(if i % 2 == 0 { 515 } else { 85 });
            power_grid.add_padded_blockage(2, trace.rect().expand(90));
            trace
                .up()
                .set_min_width()
                .vert_to_rect(data_b_pin)
                .contact_down(data_b_pin);
            power_grid.add_padded_blockage(3, trace.rect().expand(20));
        }

        let bl = read_mux
            .port(format!("bl_out_{}", i))
            .largest_rect(m2)
            .unwrap();
        let br = read_mux
            .port(format!("br_out_{}", i))
            .largest_rect(m2)
            .unwrap();
        let route_span = Span::merge([bl.hspan(), br.hspan()]);

        let m3_grid = Grid::builder()
            .line(cfg.line(3))
            .space(cfg.space(3) + 15)
            .center(Point::new(route_span.center() - cfg.line(3), 0))
            .grid(cfg.grid())
            .build()?;

        // track assignments:
        // -1 = bl / outp
        // 0 = data output
        // 1 = data input
        // 2 = br / outn

        let bl_span = m3_grid.vtrack(-1);
        let dout_span = m3_grid.vtrack(0);
        let data_span = m3_grid.vtrack(1);
        let br_span = m3_grid.vtrack(2);

        dout_spans.push(bl_span);
        dout_b_spans.push(br_span);

        let bl_vspan = Span::new(bl_bot, bl.bottom());

        let mut bl_m3 = router.trace(Rect::from_spans(bl_span, bl_vspan), 3);
        let mut br_m3 = router.trace(Rect::from_spans(br_span, bl_vspan), 3);
        power_grid.add_padded_blockage(3, bl_m3.rect());
        power_grid.add_padded_blockage(3, br_m3.rect());
        power_grid.add_padded_blockage(3, Rect::from_spans(bl_span, sa_bbox.vspan()));
        power_grid.add_padded_blockage(3, Rect::from_spans(br_span, sa_bbox.vspan()));

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
        trace.down().horiz_to_rect(dst1).down().down();

        // Route din dff to data_rect
        let src = din_dffs.port(format!("q_{}", i)).largest_rect(m2).unwrap();
        let mut trace = router.trace(src, 2);
        let voffset = if i % 2 == 0 { 1_000 } else { -800 };
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
        power_grid.add_padded_blockage(3, trace.rect());

        let mut trace = router.trace(data_rect, 3);
        trace
            .place_cursor(Dir::Vert, true)
            .vert_to(data_pin.top())
            .contact_down(data_pin);
        power_grid.add_padded_blockage(3, trace.rect());
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
        width: None,
        span: None,
    });
    power_grid.add_vdd_target(2, trace.rect());

    ////////////////////////////////////////////////////////////////////
    // Decoder routing
    ////////////////////////////////////////////////////////////////////
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

    let decoder_bus_width = bus_width(&decoder_tree.root);
    let predecoder_bus_bits = if mux_ratio == 2 {
        row_bits
    } else {
        total_addr_bits
    };

    let bus_right_edge = if let Some(ref col_decoder) = col_decoder {
        col_decoder
            .bbox()
            .union(&nand_dec.bbox())
            .into_rect()
            .left()
    } else {
        nand_dec.bbox().into_rect().left()
    };
    let track_start = grid.get_track_index(Dir::Vert, bus_right_edge, TrackLocator::EndsBefore)
        - (decoder_bus_width + 2 * predecoder_bus_bits) as isize;
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
    let addr_dff_bbox = addr_dffs.bbox();

    let track_start = track_start + decoder_bus_width as isize;
    let traces = (track_start..(track_start + 2 * predecoder_bus_bits as isize))
        .map(|track| {
            let rect = Rect::span_builder()
                .with(Dir::Vert, Span::new(addr_dff_bbox.p0.y, core_bbox.p0.y))
                .with(Dir::Horiz, grid.vtrack(track))
                .build();
            router.trace(rect, 1)
        })
        .collect::<Vec<_>>();

    for i in 0..predecoder_bus_bits {
        for (port, addr_prefix, idx) in [("q", "addr", 2 * i), ("qn", "addr_b", 2 * i + 1)] {
            let src = addr_dffs
                .port(format!("{}_{}", port, i))
                .largest_rect(m2)
                .unwrap();
            let mut trace = router.trace(src, 2);
            trace
                .place_cursor_centered()
                .horiz_to_trace(&traces[idx])
                .contact_down(traces[idx].rect());
            power_grid.add_padded_blockage(2, trace.rect().expand(cfg.space(2)));

            let (target_port, target_idx, route_at_top) = if i < decoder1_bits {
                // Route to decoder1
                (decoder1.port(format!("{}_{}", addr_prefix, i)), i, false)
            } else if i < decoder2_bits + decoder1_bits {
                // Route to decoder2
                (
                    decoder2.port(format!("{}_{}", addr_prefix, i - decoder1_bits)),
                    i - decoder1_bits,
                    false,
                )
            } else {
                // Route to column decoder
                let col_decoder = col_decoder.as_ref().unwrap();
                let idx = i - decoder1_bits - decoder2_bits;
                (
                    col_decoder.port(format!("{}_{}", addr_prefix, idx)),
                    idx,
                    true,
                )
            };
            let mut target = target_port.largest_rect(m1).unwrap();
            if route_at_top {
                let base = target.p1.y - (160 + 600 * (2 * target_idx + idx % 2) as isize);
                let bot = base - 320;
                assert!(bot >= target.p0.y);
                target.p1.y = base;
                target.p0.y = bot;
            } else {
                let base = target.p0.y + 160 + 600 * (2 * target_idx + idx % 2) as isize;
                let top = base + 320;
                assert!(top <= target.p1.y);
                target.p0.y = base;
                target.p1.y = top;
            }
            let mut trace = router.trace(target, 1);
            trace
                .place_cursor_centered()
                .up()
                .horiz_to_trace(&traces[idx])
                .contact_down(traces[idx].rect());
            power_grid.add_padded_blockage(2, trace.rect().expand(cfg.space(2)));
        }
    }

    // Route column address bit
    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: Some(ContactPosition::CenteredNonAdjacent),
            below: Some(ContactPosition::CenteredNonAdjacent),
        },
    );
    let addr_span = Span::new(
        we_control_bbox.p0.x - space - cfg.line(1),
        we_control_bbox.p0.x - space,
    );
    let addr_b_span = Span::new(
        addr_span.start() - space - cfg.line(1),
        addr_span.start() - space,
    );

    // First element is addr_0_b; second element is addr_0
    let mut addr_0_traces = Vec::with_capacity(2);
    if mux_ratio == 2 {
        for (src_port, dst_port, span) in [("qn", "sel_0", addr_b_span), ("q", "sel_1", addr_span)]
        {
            let src = addr_dffs
                .port(format!("{}_{}", src_port, total_addr_bits - 1))
                .largest_rect(m2)
                .unwrap();
            let dst = we_control.port(dst_port).largest_rect(m0).unwrap();
            let mut trace = router.trace(src, 2);
            trace.place_cursor_centered().horiz_to(span.stop());
            addr_0_traces.push(trace.rect());
            power_grid.add_padded_blockage(2, trace.rect());
            trace
                .down()
                .vert_to(dst.top())
                .down()
                .set_min_width()
                .horiz_to_rect(dst);
        }
    }

    // Route write enable (WE) to control logic
    let src = addr_dffs
        .port(format!("q_{}", total_addr_bits))
        .largest_rect(m2)
        .unwrap();
    let dst = control.port("we").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 2);
    trace.place_cursor_centered().horiz_to(dst.right() - 45);
    let blockage = trace.rect().expand(90);
    power_grid.add_padded_blockage(2, blockage);
    trace
        .set_min_width()
        .down()
        .vert_to_rect(dst)
        .contact_down(dst);

    // Route sense amp enable to sense amp clock
    let src = control.port("sense_en").largest_rect(m0).unwrap();
    let dst = sense_amp.port("clk").largest_rect(m2).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .horiz_to(src.right() - 2 * cfg.line(0))
        .up()
        .up()
        .up();
    power_grid.add_padded_blockage(2, trace.cursor_rect().expand(110));
    trace.set_width(cfg.line(3)).vert_to_rect(dst);
    power_grid.add_padded_blockage(3, trace.rect());
    trace.down().set_width(dst.height()).horiz_to_rect(dst);
    power_grid.add_padded_blockage(2, trace.rect().expand(40));

    // Route wordline enable (wl_en) from control logic to wordline drivers
    let hspan = Span::new(
        wldrv_nand_bbox.left() - space - 2 * cfg.line(1) - 40,
        wldrv_nand_bbox.left() - space,
    );
    let wl_en_rect = Rect::from_spans(hspan, wldrv_nand_bbox.vspan());
    let dst = control.port("wl_en").largest_rect(m1).unwrap();
    let mut trace = router.trace(wl_en_rect, 1);
    trace
        .set_width(2 * cfg.line(1) + 40)
        .place_cursor(Dir::Vert, false)
        .vert_to(wl_en_rect.bottom() - 3 * cfg.line(3))
        .up()
        .horiz_to_rect(dst);
    power_grid.add_padded_blockage(2, trace.rect());
    trace.up().set_min_width().vert_to_rect(dst);
    power_grid.add_padded_blockage(3, trace.rect().expand(20));
    trace.contact_down(dst).decrement_layer().contact_down(dst);
    power_grid.add_padded_blockage(2, dst.expand(50));

    // Connect wldrv_nand b inputs to wordline enable (wl_en)
    for i in 0..rows {
        let src = wldrv_nand.port(format!("b_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Horiz, false)
            .horiz_to_rect(wl_en_rect)
            .contact_up(wl_en_rect);
    }

    // Route control signals
    let grid = Grid::builder()
        .line(cfg.line(0))
        .space(space)
        .center(Point::zero())
        .grid(cfg.grid())
        .build()?;
    let track = grid.get_track_index(Dir::Vert, col_bbox.p0.x, TrackLocator::EndsBefore);

    // Write driver enable (write_driver_en)
    let src = control.port("write_driver_en").largest_rect(m0).unwrap();
    let dst = we_control.port("wr_en").largest_rect(m1).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .left_by(cfg.line(3) + cfg.space(3) + 40)
        .up()
        .up()
        .up()
        .set_min_width();
    power_grid.add_padded_blockage(2, trace.cursor_rect().expand(130));
    trace.vert_to(dst.bottom() - 500);
    power_grid.add_padded_blockage(3, trace.rect().expand(20));
    trace.down().set_min_width().horiz_to(dst.right());
    power_grid.add_padded_blockage(2, trace.rect().expand(140));
    trace.down().vert_to(dst.top());

    let (pc_b, rmux_sel_base, wmux_sel_base) = (
        track,
        track - mux_ratio as isize,
        track - 2 * mux_ratio as isize,
    );
    // precharge bar (pc_b)
    let src = pc.port("pc_b").largest_rect(m2).unwrap();
    let dst = control.port("pc_b").largest_rect(m1).unwrap();
    let mut trace = router.trace(src, 2);
    trace
        .set_width(src.height())
        .place_cursor(Dir::Horiz, false)
        .horiz_to(grid.vtrack(pc_b).start());
    power_grid.add_padded_blockage(2, trace.rect());
    trace
        .down()
        .set_min_width()
        .vert_to(dst.center().y)
        .up()
        .horiz_to_rect(dst)
        .contact_down(dst);
    power_grid.add_padded_blockage(2, trace.rect().expand(100));

    // write mux sel / write enable / write driver enable
    for i in 0..mux_ratio as isize {
        let src = we_control
            .port(format!("write_driver_en_{i}"))
            .largest_rect(m0)
            .unwrap();
        let dst = write_mux.port(format!("we_{i}")).largest_rect(m2).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Horiz, true)
            .horiz_to(grid.vtrack(wmux_sel_base + i).stop())
            .up()
            .set_min_width()
            .vert_to_rect(dst)
            .up()
            .horiz_to(dst.right());
        power_grid.add_padded_blockage(2, trace.rect().expand(100));
    }

    // read mux select
    for i in 0..mux_ratio as isize {
        let dst = read_mux.port(format!("sel_{i}")).largest_rect(m2).unwrap();
        if mux_ratio == 2 {
            let mut trace = router.trace(addr_0_traces[1 - i as usize], 2);
            trace.place_cursor(Dir::Horiz, true);
            trace.horiz_to(grid.vtrack(rmux_sel_base + i).stop());
            power_grid.add_padded_blockage(2, trace.rect().expand(90));
            trace.down().vert_to(dst.top()).up().horiz_to(dst.right());
            power_grid.add_padded_blockage(2, trace.rect().expand(90));
        } else {
            let col_decoder = col_decoder.as_ref().unwrap();
            let src = col_decoder
                .port(format!("dec_b_{i}"))
                .largest_rect(m0)
                .unwrap();
            let mut trace = router.trace(src, 0);
            let offset = if i % 2 == 0 { 1_140 } else { -1_140 };
            trace
                .place_cursor(Dir::Horiz, true)
                .up()
                .up_by(offset)
                .up()
                .set_min_width()
                .horiz_to(grid.vtrack(rmux_sel_base + i).stop());
            power_grid.add_padded_blockage(2, trace.rect().expand(90));
            trace
                .down()
                .set_min_width()
                .vert_to_rect(dst)
                .up()
                .set_min_width()
                .horiz_to_rect(dst);
            power_grid.add_padded_blockage(2, trace.rect().expand(90));
        }
    }

    // write enable control / we_control
    if mux_ratio > 2 {
        let col_decoder = col_decoder.as_ref().unwrap();
        for i in 0..mux_ratio as isize {
            let src = col_decoder
                .port(format!("dec_{i}"))
                .largest_rect(m0)
                .unwrap();
            let dst = we_control
                .port(format!("sel_{i}"))
                .largest_rect(m0)
                .unwrap();
            let mut trace = router.trace(src, 0);
            trace.place_cursor(Dir::Horiz, true).s_bend(dst, Dir::Horiz);
        }
    }

    let din_dff_bbox = din_dffs.bbox().into_rect();
    let mut blockage_hspan = col_bbox.into_rect().hspan();
    blockage_hspan.expand(true, 1_000);

    let mut spans = Vec::new();
    spans.push(din_dff_bbox.vspan());

    spans.push(Span::new(
        sense_amp.port("vss").largest_rect(m2).unwrap().bottom(),
        sense_amp.port("vdd").largest_rect(m2).unwrap().top(),
    ));
    spans.push(Span::new(
        col_inv.bbox().into_rect().bottom(),
        col_inv.port("vss").largest_rect(m2).unwrap().top(),
    ));

    power_grid.add_blockage(2, col_inv.port("vdd").largest_rect(m2).unwrap());
    spans.push(Span::new(
        write_mux.bbox().into_rect().bottom(),
        pc_bbox.top() - cfg.space(2),
    ));

    for span in spans {
        let column_blockage = Rect::from_spans(blockage_hspan, span);
        power_grid.add_padded_blockage(2, column_blockage);
    }

    power_grid.add_padded_blockage(2, addr_dff_bbox.into_rect());

    // clock (clk) distribution
    for i in 0..2 {
        let src = addr_dffs.port(format!("clk_{i}")).largest_rect(m2).unwrap();
        let mut trace = router.trace(src, 2);
        trace.place_cursor_centered().horiz_to(src.right() + 1_000);
    }

    let args = ConnectArgs::builder()
        .metal_idx(3)
        .port_idx(2)
        .router(&mut router)
        .insts(GateList::Array(&addr_dffs, total_addr_bits + 1))
        .port_name("clk")
        .dir(Dir::Vert)
        .width(cfg.line(3))
        .build()?;
    let mut clk_trace = connect(args);
    clk_trace.place_cursor(Dir::Vert, true).set_min_width();
    clk_trace.vert_to(
        std::cmp::min(din_dff_bbox.bottom(), control_bbox.bottom())
            - 3 * cfg.space(2)
            - 3 * cfg.line(3),
    );
    let clk_m3 = clk_trace.rect();
    power_grid.add_padded_blockage(3, clk_m3);
    clk_trace
        .down()
        .set_width(3 * cfg.line(2))
        .horiz_to(din_dff_bbox.right());
    power_grid.add_padded_blockage(2, clk_trace.rect());

    for i in (0..(cols / mux_ratio)).step_by(2) {
        let args = ConnectArgs::builder()
            .metal_idx(3)
            .port_idx(2)
            .router(&mut router)
            .insts(GateList::ArraySlice(&din_dffs, i, 2))
            .port_name("clk")
            .dir(Dir::Vert)
            .width(cfg.line(3))
            .build()?;
        let mut trace = connect(args);
        trace
            .place_cursor(Dir::Vert, true)
            .vert_to_trace(&clk_trace);
        power_grid.add_padded_blockage(3, trace.rect());
        trace.contact_down(clk_trace.rect());
    }

    let src = control.port("clk").largest_rect(m2).unwrap();
    let mut trace = router.trace(src, 2);
    trace
        .place_cursor_centered()
        .up()
        .set_min_width()
        .vert_to_trace(&clk_trace);
    power_grid.add_padded_blockage(3, trace.rect());
    trace.contact_down(clk_trace.rect());

    // power strapping - targets on metal 1 and metal 2
    let mut targets = vec![
        &decoder1,
        &decoder2,
        &wldrv_nand,
        &wldrv_inv,
        &nand_dec,
        &inv_dec,
        &control,
        &we_control,
        &core_pwr,
        &read_mux,
        &write_mux,
        &col_inv,
        &sense_amp,
        &din_dffs,
        &addr_dffs,
    ];
    if let Some(ref col_decoder) = col_decoder {
        targets.push(col_decoder);
    }
    for instance in targets {
        for name in ["vpb", "vdd", "vpwr"] {
            for port in instance.ports_starting_with(name) {
                if let Some(rect) = port.largest_rect(m1) {
                    power_grid.add_vdd_target(1, rect);
                }
                if let Some(rect) = port.largest_rect(m2) {
                    power_grid.add_vdd_target(2, rect);
                    power_grid.add_padded_blockage(2, rect);
                }
            }
        }
        for name in ["vnb", "vss", "vgnd", "gnd"] {
            for port in instance.ports_starting_with(name) {
                if let Some(rect) = port.largest_rect(m1) {
                    power_grid.add_gnd_target(1, rect);
                }
                if let Some(rect) = port.largest_rect(m2) {
                    power_grid.add_gnd_target(2, rect);
                    power_grid.add_padded_blockage(2, rect);
                }
            }
        }
    }

    cell.layout_mut().insts.push(core);
    cell.layout_mut().insts.push(core_pwr);
    cell.layout_mut().insts.push(decoder1);
    cell.layout_mut().insts.push(decoder2);
    cell.layout_mut().insts.push(control);
    cell.layout_mut().insts.push(we_control);
    if let Some(col_decoder) = col_decoder {
        cell.layout_mut().insts.push(col_decoder);
    }
    cell.layout_mut().insts.push(wldrv_nand);
    cell.layout_mut().insts.push(wldrv_inv);
    cell.layout_mut().insts.push(nand_dec);
    cell.layout_mut().insts.push(inv_dec);
    cell.layout_mut().insts.push(pc);
    cell.layout_mut().insts.push(read_mux);
    cell.layout_mut().insts.push(write_mux);
    cell.layout_mut().insts.push(col_inv);
    cell.layout_mut().insts.push(sense_amp.clone());
    cell.layout_mut().insts.push(din_dffs.clone());
    cell.layout_mut().insts.push(addr_dffs.clone());
    // layout.insts.push(tmc);

    let mut bbox = cell.layout().bbox().into_rect();
    // Make space for additional power straps
    bbox.p0.y -= 2_000;

    power_grid.set_enclosure(bbox);
    power_grid.add_padded_blockage(2, core_bbox.into_rect());
    power_grid.add_padded_blockage(3, core_bbox.into_rect());

    let guard_ring = draw_guard_ring(
        lib,
        GuardRingParams {
            enclosure: bbox.expand(3_000),
            prefix: "sram_guard_ring".to_string(),
        },
    )?;
    let guard_ring_inst = Instance::new("sram_guard_ring", guard_ring.cell);
    let guard_ring_bbox = guard_ring_inst.bbox().into_rect();

    // Route input and output pins
    #[allow(clippy::needless_range_loop)]
    for i in 0..(cols / mux_ratio) {
        let src = din_dffs.port(format!("d_{i}")).largest_rect(m2).unwrap();
        let offset = if i % 2 == 0 { -185 } else { 570 };
        let mut trace = router.trace(src, 2);
        let cx = src.center().x;
        trace
            .place_cursor_centered()
            .horiz_to(cx + offset)
            .up()
            .set_min_width()
            .vert_to(guard_ring_bbox.bottom());

        let rect = trace.rect();
        power_grid.add_padded_blockage(3, rect.expand(10));
        cell.add_pin(
            format!("din_{i}"),
            m3,
            Rect::from_spans(
                rect.hspan(),
                Span::new(rect.bottom(), rect.bottom() + 3 * cfg.line(3)),
            ),
        );

        // Route sense amp output
        let src = sense_amp
            .port(format!("outp_{i}"))
            .largest_rect(m2)
            .unwrap();
        let mut trace = router.trace(src, 1);

        let dout_rect = Rect::from_spans(dout_spans[i], Span::new(rect.bottom(), src.top()));
        power_grid.add_padded_blockage(3, dout_rect);
        let dout_trace = router.trace(dout_rect, 3);
        trace
            .place_cursor_centered()
            .up()
            .horiz_to_trace(&dout_trace)
            .contact_up(dout_rect);

        cell.add_pin(
            format!("dout_{i}"),
            m3,
            Rect::from_spans(
                dout_rect.hspan(),
                Span::new(dout_rect.bottom(), dout_rect.bottom() + 3 * cfg.line(3)),
            ),
        );
    }

    // Route clock (clk) pin
    let mut trace = router.trace(clk_m3, 3);
    trace
        .place_cursor(Dir::Vert, false)
        .vert_to(guard_ring_bbox.bottom());
    let clk_pin = Rect::from_spans(
        trace.rect().hspan(),
        Span::new(
            guard_ring_bbox.bottom(),
            guard_ring_bbox.bottom() + 3 * cfg.line(3),
        ),
    );
    power_grid.add_padded_blockage(3, trace.rect());
    cell.add_pin("clk", m3, clk_pin);

    // Route address and write enable pins
    for i in 0..=total_addr_bits {
        let src = addr_dffs.port(format!("d_{i}")).largest_rect(m2).unwrap();
        let mut trace = router.trace(src, 2);
        trace
            .place_cursor_centered()
            .horiz_to(guard_ring_bbox.left());

        let rect = trace.rect();
        let net = if i == total_addr_bits {
            "we".to_string()
        } else {
            format!("addr_{}", total_addr_bits - i - 1)
        };
        cell.add_pin(
            net,
            m2,
            Rect::from_spans(
                Span::new(rect.left(), rect.left() + 3 * cfg.line(2)),
                rect.vspan(),
            ),
        )
    }

    let straps = power_grid.generate()?;

    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        let (srcs, layer) = match side {
            Side::Left => (&straps.left, 2),
            Side::Right => (&straps.right, 2),
            Side::Top => (&straps.top, 3),
            Side::Bottom => (&straps.bottom, 3),
        };

        for (net, src) in srcs {
            let dst = match (side, *net) {
                (Side::Left, PowerSource::Vdd) => guard_ring.vdd_ring.left(),
                (Side::Right, PowerSource::Vdd) => guard_ring.vdd_ring.right(),
                (Side::Bottom, PowerSource::Vdd) => guard_ring.vdd_ring.bottom(),
                (Side::Top, PowerSource::Vdd) => guard_ring.vdd_ring.top(),
                (Side::Left, PowerSource::Gnd) => guard_ring.vss_ring.left(),
                (Side::Right, PowerSource::Gnd) => guard_ring.vss_ring.right(),
                (Side::Bottom, PowerSource::Gnd) => guard_ring.vss_ring.bottom(),
                (Side::Top, PowerSource::Gnd) => guard_ring.vss_ring.top(),
            };

            let width = src.span(!side.dir()).length();

            let mut trace = router.trace(*src, layer);
            trace.set_width(width).place_cursor(side.dir(), side.pos());

            match side.dir() {
                Dir::Horiz => trace.horiz_to_rect(dst),
                Dir::Vert => trace.vert_to_rect(dst),
            };
            trace.contact_down(dst);
        }
    }

    cell.add_pin("vdd", m2, guard_ring.vdd_ring.top());
    cell.add_pin("vss", m2, guard_ring.vss_ring.top());

    let routing = router.finish();

    cell.layout_mut().add_inst(straps.instance.clone());
    cell.layout_mut().add_inst(guard_ring_inst);
    cell.layout_mut().add_inst(routing);

    // Draw dnwell
    let dnwell_rect = bbox.expand(1_600);
    cell.layout_mut()
        .draw_rect(lib.pdk.get_layerkey("dnwell").unwrap(), dnwell_rect);

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    let lef = lef::generate(lef::Params {
        addr_bits: total_addr_bits,
        data_bits: cols / mux_ratio,
        cell: ptr.clone(),
        straps: &straps,
        pdk: lib.pdk.clone(),
    });

    Ok(PhysicalDesign { cell: ptr, lef })
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
    #[builder(setter(strip_option), default)]
    pub(crate) width: Option<isize>,
    #[builder(setter(strip_option), default)]
    pub(crate) span: Option<Span>,
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
    /// Instance, start, width
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

    let width = args.width.unwrap_or(3 * cfg.line(args.metal_idx));

    let target_area = Rect::from(port_start.union(&port_stop));
    let mut span = args.span.unwrap_or_else(|| target_area.span(args.dir));
    let trace_xspan =
        Span::from_center_span_gridded(target_area.span(!args.dir).center(), width, cfg.grid());

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

    use crate::utils::{panic_on_err, test_lef_path, test_path};

    use super::*;

    #[test]
    fn test_sram_bank_32x32m2() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_32x32m2")?;
        let PhysicalDesign { cell: _, lef } = draw_sram_bank(
            &mut lib,
            SramBankParams {
                rows: 32,
                cols: 32,
                mux_ratio: 2,
                wmask_groups: 1,
            },
        )
        .map_err(panic_on_err)?;
        lef.save(test_lef_path(&lib)).expect("failed to export LEF");

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }

    #[test]
    fn test_sram_bank_32x32m4() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_32x32m4")?;
        let PhysicalDesign { cell: _, lef } = draw_sram_bank(
            &mut lib,
            SramBankParams {
                rows: 32,
                cols: 32,
                mux_ratio: 4,
                wmask_groups: 1,
            },
        )
        .map_err(panic_on_err)?;
        lef.save(test_lef_path(&lib)).expect("failed to export LEF");

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }

    #[test]
    fn test_sram_bank_32x32m8() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_32x32m8")?;
        let PhysicalDesign { cell: _, lef } = draw_sram_bank(
            &mut lib,
            SramBankParams {
                rows: 32,
                cols: 32,
                mux_ratio: 8,
                wmask_groups: 1,
            },
        )
        .map_err(panic_on_err)?;
        lef.save(test_lef_path(&lib)).expect("failed to export LEF");

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }

    #[test]
    fn test_sram_bank_128x64m2() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_128x64m2")?;
        draw_sram_bank(
            &mut lib,
            SramBankParams {
                rows: 128,
                cols: 64,
                mux_ratio: 2,
                wmask_groups: 1,
            },
        )
        .map_err(panic_on_err)?;

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }

    #[test]
    fn test_sram_bank_64x128m2() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_64x128m2")?;
        draw_sram_bank(
            &mut lib,
            SramBankParams {
                rows: 64,
                cols: 128,
                mux_ratio: 2,
                wmask_groups: 1,
            },
        )
        .map_err(panic_on_err)?;

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }

    #[test]
    fn test_sram_bank_16x16m2() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank_16x16m2")?;
        draw_sram_bank(
            &mut lib,
            SramBankParams {
                rows: 16,
                cols: 16,
                mux_ratio: 2,
                wmask_groups: 1,
            },
        )
        .map_err(panic_on_err)?;

        lib.save_gds(test_path(&lib)).map_err(panic_on_err)?;

        Ok(())
    }
}
