use anyhow::anyhow;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Element, Instance, Int, Layout, Point, Rect,
    Shape, Span, TransformTrait,
};
use layout21::utils::Ptr;
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::mos::{Intent, MosDevice, MosParams, MosType};
use pdkprims::PdkLib;

use crate::layout::array::*;
use crate::layout::bank::{connect, ConnectArgs};
use crate::layout::route::grid::{Grid, TrackLocator};
use crate::layout::route::{ContactBounds, Router, VertDir};
use crate::tech::BITCELL_WIDTH;
use crate::Result;

use crate::layout::bank::GateList;
use crate::layout::common::{
    draw_two_level_contact, MergeArgs, TwoLevelContactParams, NWELL_COL_SIDE_EXTEND,
    NWELL_COL_VERT_EXTEND,
};

pub struct WriteMuxParams {
    width: isize,
    wmask: bool,
}

pub fn draw_write_mux(lib: &mut PdkLib, params: WriteMuxParams) -> Result<Ptr<Cell>> {
    let name = "write_mux";

    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);
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

    mos_bls.align_above(mos_we.bbox(), 1_000);

    let mut port = mos_we.port("sd_0_0");
    port.set_net("vss");
    abs.add_port(port);

    let mut port = mos_we.port("gate_0");
    port.set_net("we");
    abs.add_port(port);

    layout.insts.push(mos_we.clone());

    layout.insts.push(mos_bls.clone());

    let mut port = mos_bls.port("gate_0");
    port.set_net("data");
    abs.add_port(port);

    let mut port = mos_bls.port("gate_1");
    port.set_net("data_b");
    abs.add_port(port);

    let mut trace = router.trace(mos_bls.port("sd_0_1").largest_rect(m0).unwrap(), 0);
    trace
        .contact_up(trace.rect())
        .increment_layer()
        .place_cursor(Dir::Vert, false);

    let dst = mos_we.port("sd_0_1").largest_rect(m0).unwrap();
    trace
        .vert_to(dst.bottom())
        .contact_on(dst, VertDir::Below, ContactBounds::FitOne(m0, dst))
        .decrement_layer();

    let mut trace = router.trace(mos_bls.port("sd_0_0").largest_rect(m0).unwrap(), 0);
    trace.contact_up(trace.rect());
    let mut port = AbstractPort::new("br");
    port.add_shape(m1, Shape::Rect(trace.rect()));
    abs.add_port(port);

    let mut trace = router.trace(mos_bls.port("sd_0_2").largest_rect(m0).unwrap(), 0);
    trace.contact_up(trace.rect());
    let mut port = AbstractPort::new("bl");
    port.add_shape(m1, Shape::Rect(trace.rect()));
    abs.add_port(port);

    layout.insts.push(router.finish());

    let cell = Cell {
        name: name.to_string(),
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

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
        let src = core_inst.port(format!("vss_{i}")).largest_rect(m0).unwrap();
        let dst = tap_inst
            .port(format!("vss_{}", (i + 1) / 2))
            .largest_rect(m0)
            .unwrap();
        let dst = router.trace(dst, 0);
        let mut trace = router.trace(src, 0);
        trace.place_cursor_centered().horiz_to_trace(&dst);
        span = trace.rect().vspan();

        cell.add_pin_from_port(core_inst.port(format!("bl_{i}")), m1);
        cell.add_pin_from_port(core_inst.port(format!("br_{i}")), m1);
    }

    let start = tap_inst.port("vss_0").largest_rect(m1).unwrap();
    let end = tap_inst
        .port(format!("vss_{}", width / 2))
        .largest_rect(m1)
        .unwrap();

    let length = span.length();
    span.expand(true, length).expand(false, length);

    let rect = Rect::span_builder()
        .with(Dir::Horiz, Span::new(start.left(), end.right()))
        .with(Dir::Vert, span)
        .build();

    let mut trace = router.trace(rect, 2);

    for i in 0..(width / 2 + 1) {
        let target = tap_inst.port(format!("vss_{i}")).largest_rect(m0).unwrap();
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

    let data = core_inst.port("data_0").largest_rect(m0).unwrap();
    let data_track = grid.get_track_index(Dir::Horiz, data.bottom(), TrackLocator::EndsBefore);
    let data_b_track = data_track - 1;

    for (idx, i) in (0..width).step_by(mux_ratio).enumerate() {
        for port in ["data", "data_b"] {
            let track = match port {
                "data" => data_track,
                "data_b" => data_b_track,
                _ => unreachable!(),
            };
            let start = core_inst
                .port(format!("{}_{}", port, i))
                .largest_rect(m0)
                .unwrap();
            let stop = core_inst
                .port(format!("{}_{}", port, i + mux_ratio - 1))
                .largest_rect(m0)
                .unwrap();
            let mut hspan = Span::new(start.left(), stop.right());
            hspan.expand(true, 400).expand(false, 400);
            let rect = Rect::from_spans(hspan, grid.track(Dir::Horiz, track));

            cell.add_pin(format!("{}_{}", port, idx), m2, rect);

            let data = router.trace(rect, 2);

            for delta in 0..mux_ratio {
                let src = core_inst
                    .port(format!("{}_{}", port, i + delta))
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

    let bits_per_wmask = width / (mux_ratio * wmask);

    for i in 0..mux_ratio {
        for j in 0..wmask {
            let idxs = ((bits_per_wmask * mux_ratio * j + i)
                ..(bits_per_wmask * mux_ratio * (j + 1) + i))
                .step_by(mux_ratio)
                .collect::<Vec<_>>();
            assert_eq!(idxs.len(), bits_per_wmask);

            let start = idxs[0];
            let stop = idxs[idxs.len() - 1];

            let start = core_inst
                .port(format!("we_{}", start))
                .largest_rect(m0)
                .unwrap();
            let stop = core_inst
                .port(format!("we_{}", stop))
                .largest_rect(m0)
                .unwrap();

            let mut hspan = Span::new(start.left(), stop.right());
            hspan.expand(true, 100).expand(false, 100);

            let track = track - i as isize;
            let rect = Rect::from_spans(hspan, grid.htrack(track));

            cell.add_pin(format!("we_{}_{}", i, j), m2, rect);
            let we = router.trace(rect, 2);

            for idx in idxs {
                let src = core_inst
                    .port(format!("we_{}", idx))
                    .largest_rect(m0)
                    .unwrap();
                let mut trace = router.trace(src, 0);

                trace
                    .place_cursor(Dir::Vert, false)
                    .vert_to_trace(&we)
                    .contact_up(we.rect())
                    .increment_layer()
                    .contact_up(we.rect());
            }
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
