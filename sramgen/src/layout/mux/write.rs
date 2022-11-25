use anyhow::anyhow;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, BoundBoxTrait, Cell, Instance, Int, Layout, Point, Rect, Span};
use layout21::utils::Ptr;
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::mos::{Intent, MosDevice, MosParams, MosType};
use pdkprims::PdkLib;

use crate::layout::array::*;

use crate::layout::route::grid::{Grid, TrackLocator};
use crate::layout::route::{ContactBounds, Router, VertDir};
use crate::layout::sram::{connect, ConnectArgs, GateList};
use crate::tech::BITCELL_WIDTH;
use crate::{bus_bit, Result};

pub struct WriteMuxParams {
    pub width: isize,
    pub wmask: bool,
}

pub fn draw_write_mux(lib: &mut PdkLib, params: WriteMuxParams) -> Result<Ptr<Cell>> {
    let WriteMuxParams { wmask, .. } = params;

    let name = "write_mux";

    let mut cell = Cell::empty(name);
    let mut router = Router::new("write_mux_route", lib.pdk.clone());
    let m0 = lib.pdk.metal(0);
    let m1 = lib.pdk.metal(1);

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

    let mos_we = Instance::builder()
        .inst_name("mos_we")
        .cell(ptx.cell.clone())
        .angle(90f64)
        .build()?;

    let (bbox, wmask_inst) = if wmask {
        let mut mos_wmask = Instance::builder()
            .inst_name("mos_wmask")
            .cell(ptx.cell.clone())
            .angle(90f64)
            .build()?;
        let bbox = mos_we.bbox();
        mos_wmask.align_centers_horizontally_gridded(bbox, lib.pdk.grid());
        mos_wmask.align_above(bbox, 1_000);
        (mos_wmask.bbox(), Some(mos_wmask))
    } else {
        (mos_we.bbox(), None)
    };

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

    let mut mos_bls = Instance::builder()
        .inst_name("mos_bls")
        .cell(ptx.cell.clone())
        .angle(90f64)
        .build()?;

    mos_bls.align_above(bbox, 2_100);
    mos_bls.align_centers_horizontally_gridded(bbox, lib.pdk.grid());

    cell.add_pin_from_port(mos_we.port("sd_0_0").named("vss"), m0);
    cell.add_pin_from_port(mos_we.port("gate_0").named("we"), m0);
    cell.add_pin_from_port(mos_bls.port("gate_0").named("data"), m0);
    cell.add_pin_from_port(mos_bls.port("gate_1").named("data_b"), m0);

    cell.layout_mut().insts.push(mos_we.clone());
    cell.layout_mut().insts.push(mos_bls.clone());

    let mut trace = router.trace(mos_bls.port("sd_0_1").largest_rect(m0).unwrap(), 0);
    trace
        .contact_up(trace.rect())
        .increment_layer()
        .place_cursor(Dir::Vert, false);

    let dst = if let Some(ref mos_wmask) = wmask_inst {
        mos_wmask.port("sd_0_1").largest_rect(m0).unwrap()
    } else {
        mos_we.port("sd_0_1").largest_rect(m0).unwrap()
    };
    trace
        .down_by(2_200)
        .s_bend(dst, Dir::Vert)
        .contact_on(dst, VertDir::Below, ContactBounds::FitOne(m0, dst))
        .decrement_layer();

    if let Some(ref mos_wmask) = wmask_inst {
        let src = mos_wmask.port("sd_0_0").largest_rect(m0).unwrap();
        let dst = mos_we.port("sd_0_1").largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .contact_up(trace.rect())
            .increment_layer()
            .place_cursor(Dir::Vert, false)
            .down_by(900)
            .s_bend(dst, Dir::Vert)
            .contact_on(dst, VertDir::Below, ContactBounds::FitOne(m0, dst));

        let mut trace = router.trace(mos_wmask.port("gate_0").largest_rect(m0).unwrap(), 0);
        trace
            .place_cursor(Dir::Vert, false)
            .left_by(200)
            .down_by(400)
            .up();
        cell.add_pin("wmask", m1, trace.cursor_rect());
    }

    let mut trace = router.trace(mos_bls.port("sd_0_0").largest_rect(m0).unwrap(), 0);
    trace.contact_up(trace.rect());
    cell.add_pin("br", m1, trace.rect());

    let mut trace = router.trace(mos_bls.port("sd_0_2").largest_rect(m0).unwrap(), 0);
    trace.contact_up(trace.rect());
    cell.add_pin("bl", m1, trace.rect());

    if let Some(wmask_inst) = wmask_inst {
        cell.layout_mut().add_inst(wmask_inst);
    }

    cell.layout_mut().insts.push(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

/// Draws an array of write muxes
///
/// The wmask parameter represents the number of write mask groups.
///
/// For example, if width = 64, mux_ratio = 2, and wmask = 4,
/// then there will be 2 32-bit words, with a 4 bit write mask
/// enabling byte write.
pub fn draw_write_mux_array(
    lib: &mut PdkLib,
    width: usize,
    mux_ratio: usize,
    wmask: usize,
) -> Result<Ptr<Cell>> {
    assert!(width >= 2);
    assert_eq!(width % 2, 0);

    let mux = draw_write_mux(
        lib,
        WriteMuxParams {
            width: BITCELL_WIDTH,
            wmask: wmask > 1,
        },
    )?;
    let muxes = draw_cell_array(
        ArrayCellParams {
            name: "write_mux_core_array".to_string(),
            num: width,
            cell: mux,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let bbox = muxes.cell.read().unwrap().layout.as_ref().unwrap().bbox();
    let tap = draw_write_mux_tap_cell(lib, bbox.height())?;

    let taps = draw_cell_array(
        ArrayCellParams {
            name: "write_mux_tap_array".to_string(),
            num: width / 2 + 1,
            cell: tap,
            spacing: Some(2 * 2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let name = "write_mux_array";
    let mut cell = Cell::empty(name);

    let core_inst = Instance::new("write_mux_core_array", muxes.cell);
    let mut tap_inst = Instance::new("write_mux_tap_array", taps.cell);
    tap_inst.align_centers_gridded(core_inst.bbox(), lib.pdk.grid());

    let mut router = Router::new("write_mux_array_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    let mut span = Span::new(0, 0);

    for i in 0..width {
        let src = core_inst.port(bus_bit("vss", i)).largest_rect(m0).unwrap();
        let dst = tap_inst
            .port(bus_bit("vss", (i + 1) / 2))
            .largest_rect(m0)
            .unwrap();
        let dst = router.trace(dst, 0);
        let mut trace = router.trace(src, 0);
        trace.place_cursor_centered().horiz_to_trace(&dst);
        span = trace.rect().vspan();

        cell.add_pin_from_port(core_inst.port(bus_bit("bl", i)), m1);
        cell.add_pin_from_port(core_inst.port(bus_bit("br", i)), m1);
    }

    let start = tap_inst.port(bus_bit("vss", 0)).largest_rect(m1).unwrap();
    let end = tap_inst
        .port(bus_bit("vss", width / 2))
        .largest_rect(m1)
        .unwrap();

    let length = span.length();
    span.expand(true, length).expand(false, length);

    let rect = Rect::span_builder()
        .with(Dir::Horiz, Span::new(start.left() - 100, end.right() + 100))
        .with(Dir::Vert, span)
        .build();

    let mut trace = router.trace(rect, 2);

    for i in 0..(width / 2 + 1) {
        let target = tap_inst.port(bus_bit("vss", i)).largest_rect(m0).unwrap();
        trace.contact_on(
            target.intersection(&trace.rect().into()).into_rect(),
            VertDir::Below,
            ContactBounds::Minimum,
        );
    }

    cell.add_pin("vss", m2, rect);

    cell.layout_mut().add_inst(core_inst.clone());
    cell.layout_mut().add_inst(tap_inst);
    let bbox = cell.layout_mut().bbox().into_rect();
    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    // Route gate signals
    let space = lib.pdk.bus_min_spacing(
        2,
        cfg.line(2),
        ContactPolicy {
            above: Some(ContactPosition::CenteredNonAdjacent),
            below: Some(ContactPosition::CenteredNonAdjacent),
        },
    );
    let grid = Grid::builder()
        .line(3 * cfg.line(2))
        .space(space)
        .center(Point::zero())
        .grid(tc.grid)
        .build()?;

    let data = core_inst.port(bus_bit("data", 0)).largest_rect(m0).unwrap();
    let data_track = grid.get_track_index(Dir::Horiz, data.bottom(), TrackLocator::EndsBefore);
    let data_b_track = data_track - 1;

    for (idx, i) in (0..width).step_by(mux_ratio).enumerate() {
        for port in ["data", "data_b"] {
            let track = match port {
                "data" => data_track,
                "data_b" => data_b_track,
                _ => unreachable!(),
            };
            let start = core_inst.port(bus_bit(port, i)).largest_rect(m0).unwrap();
            let stop = core_inst
                .port(bus_bit(port, i + mux_ratio - 1))
                .largest_rect(m0)
                .unwrap();
            let mut hspan = Span::new(start.left(), stop.right());
            hspan.expand(true, 400).expand(false, 400);
            let rect = Rect::from_spans(hspan, grid.track(Dir::Horiz, track));

            cell.add_pin(bus_bit(port, idx), m2, rect);

            let data = router.trace(rect, 2);

            for delta in 0..mux_ratio {
                let src = core_inst
                    .port(bus_bit(port, i + delta))
                    .largest_rect(m0)
                    .unwrap();
                let mut trace = router.trace(src, 0);
                let offset = match (port, delta % 2) {
                    ("data", 0) | ("data_b", 1) => src.left() - 280,
                    ("data", 1) | ("data_b", 0) => src.right() + 280,
                    _ => unreachable!(),
                };
                trace
                    .place_cursor(Dir::Vert, false)
                    .horiz_to(offset)
                    .vert_to_trace(&data)
                    .contact_up(data.rect())
                    .increment_layer()
                    .contact_up(data.rect());
            }
        }
    }

    let track = grid.get_track_index(Dir::Horiz, bbox.bottom(), TrackLocator::EndsBefore);

    assert_eq!(
        width % (mux_ratio * wmask),
        0,
        "Width must be divisible by mux_ratio * wmask"
    );

    let start = core_inst.port(bus_bit("we", 0)).largest_rect(m0).unwrap();
    let end = core_inst
        .port(bus_bit("we", width - 1))
        .largest_rect(m0)
        .unwrap();
    let mut hspan = Span::new(start.left(), end.right());
    hspan.expand(true, 200).expand(false, 200);

    for i in 0..mux_ratio {
        let track = track - i as isize;
        let rect = Rect::from_spans(hspan, grid.htrack(track));

        let we = router.trace(rect, 2);
        cell.add_pin(bus_bit("we", i), m2, rect);

        for j in (i..width).step_by(mux_ratio) {
            let src = core_inst.port(bus_bit("we", j)).largest_rect(m0).unwrap();
            let mut trace = router.trace(src, 0);

            trace
                .place_cursor(Dir::Vert, false)
                .vert_to_trace(&we)
                .contact_up(we.rect())
                .increment_layer()
                .contact_up(we.rect());
        }
    }

    if wmask > 1 {
        // Number of columns controlled by one write mask bit
        let wmask_width = width / wmask;

        for i in 0..wmask {
            let args = ConnectArgs::builder()
                .metal_idx(2)
                .port_idx(1)
                .router(&mut router)
                .insts(GateList::ArraySlice(
                    &core_inst,
                    i * wmask_width,
                    wmask_width,
                ))
                .port_name("wmask")
                .dir(Dir::Horiz)
                .overhang(200)
                .build()?;
            let trace = connect(args);
            cell.add_pin(bus_bit("wmask", i), m2, trace.rect());
        }
    }

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

fn draw_write_mux_tap_cell(lib: &mut PdkLib, height: Int) -> Result<Ptr<Cell>> {
    let name = "write_mux_tap_cell";
    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);

    let m0 = lib.pdk.metal(0);
    let m1 = lib.pdk.metal(1);

    let tap = lib
        .pdk
        .get_contact_sized("ptap", Dir::Vert, m0, height)
        .ok_or_else(|| anyhow!("Failed to generate contact of correct size"))?;
    let ct = lib
        .pdk
        .get_contact_sized("viali", Dir::Vert, m1, height)
        .ok_or_else(|| anyhow!("Failed to generate contact of correct size"))?;

    let tap_inst = Instance::builder()
        .inst_name("tap")
        .cell(tap.cell.clone())
        .build()?;
    let mut ct_inst = Instance::builder()
        .inst_name("contact")
        .cell(ct.cell.clone())
        .build()?;
    ct_inst.align_centers_gridded(tap_inst.bbox(), lib.pdk.grid());

    let mut port = ct_inst.port("x");
    port.set_net("vss");
    abs.add_port(port);

    layout.insts.push(tap_inst);
    layout.insts.push(ct_inst);

    let cell = Cell {
        name: name.into(),
        layout: Some(layout),
        abs: Some(abs),
    };

    Ok(Ptr::new(cell))
}
