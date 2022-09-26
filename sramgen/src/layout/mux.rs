use anyhow::anyhow;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, AbstractPort, Element, Int, Rect, Shape, Span, TransformTrait};
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
use crate::layout::route::{ContactBounds, Router, VertDir};
use crate::Result;

use super::bank::GateList;
use super::common::{MergeArgs, NWELL_COL_SIDE_EXTEND, NWELL_COL_VERT_EXTEND};

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

    let mut vpb = ptx.merged_vpb_port(0).transform(&mos1.transform());
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

pub fn draw_read_mux_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let name = "read_mux_array";
    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);
    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    let mux = draw_read_mux(lib)?;

    let cell = draw_cell_array(
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

    let inst = Instance::new("read_mux_array_core", cell.cell);

    for port in inst.ports() {
        abs.add_port(port);
    }

    layout.insts.push(inst.clone());
    let bbox = layout.bbox().into_rect();

    let mut router = Router::new("read_mux_array_route", lib.pdk.clone());

    // Route gate signals
    let grid = Grid::builder()
        .line(3 * tc.layer("m2").width)
        .space(tc.layer("m2").space)
        .center(Point::zero())
        .grid(tc.grid)
        .build()?;

    let track = grid.get_track_index(Dir::Horiz, bbox.bottom(), TrackLocator::EndsBefore);
    let rect = Rect::span_builder()
        .with(Dir::Vert, grid.htrack(track))
        .with(Dir::Horiz, bbox.hspan())
        .build();
    let sel_m2 = router.trace(rect, 2);

    let rect = Rect::span_builder()
        .with(Dir::Vert, grid.htrack(track - 1))
        .with(Dir::Horiz, bbox.hspan())
        .build();
    let sel_b_m2 = router.trace(rect, 2);

    for i in 0..width {
        let m0 = router.cfg().layerkey(0);
        let src = inst.port(format!("sel_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Vert, false)
            .vert_to(sel_m2.rect().bottom())
            .contact_up(sel_m2.rect())
            .increment_layer()
            .contact_up(sel_m2.rect());
        let src = inst.port(format!("sel_b_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Vert, false)
            .vert_to(sel_b_m2.rect().bottom())
            .contact_up(sel_b_m2.rect())
            .increment_layer()
            .contact_up(sel_b_m2.rect());
    }

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();

    let vpb = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Array(&inst, width))
        .port_name("vpb")
        .top_overhang(NWELL_COL_VERT_EXTEND)
        .bot_overhang(NWELL_COL_VERT_EXTEND)
        .left_overhang(NWELL_COL_SIDE_EXTEND)
        .right_overhang(NWELL_COL_SIDE_EXTEND)
        .build()?
        .element();

    layout.add(vpb);

    layout.insts.push(router.finish());

    Ok(Ptr::new(Cell {
        name: name.into(),
        layout: Some(layout),
        abs: Some(abs),
    }))
}

pub fn draw_write_mux(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
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

    let mos_gnd = Instance::builder()
        .inst_name("mos_1")
        .cell(ptx.cell.clone())
        .angle(90f64)
        .build()?;

    let mut port = mos_gnd.port("sd_0_0");
    port.set_net("vss");
    abs.add_port(port);

    let mut port = mos_gnd.port("gate_0");
    port.set_net("we");
    abs.add_port(port);

    layout.insts.push(mos_gnd.clone());

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
        .inst_name("mos_2")
        .cell(ptx.cell.clone())
        .angle(90f64)
        .build()?;

    mos_bls.align_above(mos_gnd.bbox(), 1_000);

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

    let dst = mos_gnd.port("sd_0_1").largest_rect(m0).unwrap();
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

pub fn draw_write_mux_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    assert!(width >= 2);
    assert_eq!(width % 2, 0);

    let mux = draw_write_mux(lib)?;
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
    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);

    let core_inst = Instance::new("write_mux_core_array", muxes.cell);
    let mut tap_inst = Instance::new("write_mux_tap_array", taps.cell);
    tap_inst.align_centers_gridded(core_inst.bbox(), lib.pdk.grid());

    let mut router = Router::new("write_mux_array_route", lib.pdk.clone());
    let m0 = router.cfg().layerkey(0);
    let m1 = router.cfg().layerkey(1);
    let m2 = router.cfg().layerkey(2);

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

        abs.add_port(core_inst.port(format!("bl_{i}")));
        abs.add_port(core_inst.port(format!("br_{i}")));
        abs.add_port(core_inst.port(format!("data_{i}")));
        abs.add_port(core_inst.port(format!("data_b_{i}")));
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

    let mut port = AbstractPort::new("vss");
    port.add_shape(m2, Shape::Rect(rect));
    abs.add_port(port);

    layout.add_inst(core_inst.clone());
    layout.add_inst(tap_inst);
    let bbox = layout.bbox().into_rect();
    let tc = lib.pdk.config();
    let tc = tc.read().unwrap();

    // Route gate signals
    let grid = Grid::builder()
        .line(tc.layer("m2").width)
        .space(tc.layer("m2").space)
        .center(Point::zero())
        .grid(tc.grid)
        .build()?;

    let track = grid.get_track_index(Dir::Horiz, bbox.bottom(), TrackLocator::EndsBefore);
    let rect = Rect::span_builder()
        .with(Dir::Vert, grid.htrack(track))
        .with(Dir::Horiz, bbox.hspan())
        .build();
    let we0_m2 = router.trace(rect, 2);

    let rect = Rect::span_builder()
        .with(Dir::Vert, grid.htrack(track - 1))
        .with(Dir::Horiz, bbox.hspan())
        .build();
    let we1_m2 = router.trace(rect, 2);

    for i in 0..width {
        let m0 = router.cfg().layerkey(0);
        let src = core_inst.port(format!("we_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        let target = if i % 2 == 0 {
            we0_m2.rect()
        } else {
            we1_m2.rect()
        };
        trace
            .place_cursor(Dir::Vert, false)
            .vert_to(target.bottom())
            .contact_up(target)
            .increment_layer()
            .contact_up(target);
    }

    layout.add_inst(router.finish());

    Ok(Ptr::new(Cell {
        name: name.into(),
        layout: Some(layout),
        abs: Some(abs),
    }))
}

fn draw_write_mux_tap_cell(lib: &mut PdkLib, height: Int) -> Result<Ptr<Cell>> {
    let name = "write_mux_tapcell";
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
