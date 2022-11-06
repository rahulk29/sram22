use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Element, Instance, Layout, Point, Rect, Shape,
    Span, TransformTrait,
};
use layout21::utils::Ptr;

use pdkprims::mos::{Intent, MosDevice, MosParams, MosType};
use pdkprims::PdkLib;

use crate::layout::array::*;
use crate::layout::bank::{connect, ConnectArgs};
use crate::layout::route::grid::{Grid, TrackLocator};
use crate::layout::route::Router;

use crate::Result;

use crate::layout::bank::GateList;
use crate::layout::common::{
    draw_two_level_contact, MergeArgs, TwoLevelContactParams, NWELL_COL_SIDE_EXTEND,
    NWELL_COL_VERT_EXTEND,
};

pub fn draw_read_mux(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "read_mux".to_string();

    let mut layout = Layout::new(&name);
    let mut abs = Abstract::new(&name);

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

    let vpb1 = ptx.merged_vpb_port(0).transform(&mos1.transform());

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

    let mut vpb = ptx.merged_vpb_port(0).transform(&mos2.transform());
    vpb.merge(vpb1);
    abs.add_port(vpb);

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
    let mut router = Router::new("read_mux_route", lib.pdk.clone());
    let mut traces = Vec::with_capacity(5);

    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    for (i, port) in [
        (-track - 2, "br_0"),
        (-track, "bl_0"),
        (-1, "bl_out"),
        (1, "br_out"),
        (track, "bl_1"),
        (track + 2, "br_1"),
    ] {
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
        let mut port = AbstractPort::new(port);
        port.add_shape(m1, Shape::Rect(rect));
        abs.add_port(port);
    }

    let mut port = mos1.port("gate_0");
    port.set_net("sel");
    abs.add_port(port);

    let mut port = mos2.port("gate_0");
    port.set_net("sel_b");
    abs.add_port(port);

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
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_read_mux_array(
    lib: &mut PdkLib,
    mut width: usize,
    mut mux_ratio: usize,
) -> Result<Ptr<Cell>> {
    assert_eq!(mux_ratio % 2, 0);
    assert!(mux_ratio >= 2);

    // Divide mux ratio by 2, since read muxes are internally 2:1
    mux_ratio /= 2;
    width /= 2;

    let name = "read_mux_array";
    let mut cell = Cell::empty(name);

    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    let mux = draw_read_mux(lib)?;
    let tap = draw_read_mux_tap_cell(lib)?;

    let array = draw_cell_array(
        ArrayCellParams {
            name: "read_mux_array_core".to_string(),
            num: width,
            cell: mux,
            spacing: Some(2_500 * 2),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let taps = draw_cell_array(
        ArrayCellParams {
            name: "read_mux_array_taps".to_string(),
            num: width + 1,
            cell: tap,
            spacing: Some(2_500 * 2),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let mut router = Router::new("read_mux_array_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    let inst = Instance::new("read_mux_array_core", array.cell);
    for i in 0..width {
        cell.add_pin_from_port(
            inst.port(format!("bl_0_{i}"))
                .named(format!("bl_{}", 2 * i)),
            m1,
        );
        cell.add_pin_from_port(
            inst.port(format!("bl_1_{i}"))
                .named(format!("bl_{}", 2 * i + 1)),
            m1,
        );
        cell.add_pin_from_port(
            inst.port(format!("br_0_{i}"))
                .named(format!("br_{}", 2 * i)),
            m1,
        );
        cell.add_pin_from_port(
            inst.port(format!("br_1_{i}"))
                .named(format!("br_{}", 2 * i + 1)),
            m1,
        );
    }
    let mut tap_inst = Instance::new("read_mux_array_taps", taps.cell);
    tap_inst.align_centers_gridded(inst.bbox(), lib.pdk.grid());

    for i in 0..width {
        for port in [
            format!("bl_0_{i}"),
            format!("bl_1_{i}"),
            format!("br_0_{i}"),
            format!("br_1_{i}"),
        ] {
            cell.abs_mut().add_port(inst.port(port));
        }
    }

    cell.layout_mut().insts.push(inst.clone());
    cell.layout_mut().insts.push(tap_inst.clone());
    let bbox = cell.layout_mut().bbox().into_rect();

    // Route gate signals
    let grid = Grid::builder()
        .line(3 * tc.layer("m2").width)
        .space(tc.layer("m2").space)
        .center(Point::zero())
        .grid(tc.grid)
        .build()?;

    let track = grid.get_track_index(Dir::Horiz, bbox.bottom(), TrackLocator::EndsBefore);
    let sel_tracks = (track - 2 * mux_ratio as isize + 1..=track)
        .map(|i| Rect::from_spans(bbox.hspan(), grid.htrack(i)))
        .map(|rect| router.trace(rect, 2))
        .collect::<Vec<_>>();

    for i in 0..width {
        let sel = &sel_tracks[2 * i % (2 * mux_ratio)];
        let src = inst.port(format!("sel_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Vert, false)
            .vert_to(sel.rect().bottom())
            .contact_up(sel.rect())
            .increment_layer()
            .contact_up(sel.rect());

        let sel = &sel_tracks[(2 * i + 1) % (2 * mux_ratio)];
        let src = inst.port(format!("sel_b_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Vert, false)
            .vert_to(sel.rect().bottom())
            .contact_up(sel.rect())
            .increment_layer()
            .contact_up(sel.rect());
    }

    for (i, trace) in sel_tracks.into_iter().enumerate() {
        cell.add_pin(format!("sel_{}", i), m2, trace.rect());
    }

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();

    let vpb = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Array(&inst, width))
        .port_name("vpb")
        .top_overhang(NWELL_COL_VERT_EXTEND)
        .bot_overhang(NWELL_COL_VERT_EXTEND)
        .left_overhang(NWELL_COL_SIDE_EXTEND + 800)
        .right_overhang(NWELL_COL_SIDE_EXTEND + 800)
        .build()?
        .element();

    let args = ConnectArgs::builder()
        .metal_idx(2)
        .port_idx(1)
        .router(&mut router)
        .insts(GateList::Array(&tap_inst, width + 1))
        .port_name("x")
        .dir(Dir::Horiz)
        .overhang(100)
        .build()?;
    let trace = connect(args);
    cell.add_pin("vdd", m2, trace.rect());

    assert_eq!(width % mux_ratio, 0);
    for i in (0..width).step_by(mux_ratio) {
        let args = ConnectArgs::builder()
            .metal_idx(2)
            .port_idx(1)
            .router(&mut router)
            .insts(GateList::ArraySlice(&inst, i, mux_ratio))
            .port_name("bl_out")
            .dir(Dir::Horiz)
            .overhang(100)
            .transverse_offset(800)
            .build()?;
        let trace = connect(args);
        cell.add_pin(format!("bl_out_{}", i / mux_ratio), m2, trace.rect());

        let args = ConnectArgs::builder()
            .metal_idx(2)
            .port_idx(1)
            .router(&mut router)
            .insts(GateList::ArraySlice(&inst, i, mux_ratio))
            .port_name("br_out")
            .dir(Dir::Horiz)
            .overhang(100)
            .transverse_offset(-800)
            .build()?;
        let trace = connect(args);
        cell.add_pin(format!("br_out_{}", i / mux_ratio), m2, trace.rect());
    }

    cell.layout_mut().add(vpb);
    cell.layout_mut().insts.push(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_read_mux_tap_cell(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let params = TwoLevelContactParams::builder()
        .name("read_mux_tap_cell")
        .bot_stack("ntap")
        .top_stack("viali")
        .bot_rows(7)
        .top_rows(6)
        .build()?;
    let contact = draw_two_level_contact(lib, params)?;
    Ok(contact)
}
