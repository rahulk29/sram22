//! Timing multiplier circuit layout.

use anyhow::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Dir, Instance, Layout, Rect, Shape, Span,
};
use layout21::utils::Ptr;
use pdkprims::bus::ContactPolicy;
use pdkprims::PdkLib;

use crate::config::gate::{GateParams, Size};
use crate::config::tmc::{TmcParams, TmcUnitParams};

use super::gate::{draw_inv, draw_nand2};
use super::route::Router;

pub fn draw_dbdr_delay_cell(lib: &mut PdkLib, name: &str) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);

    let nand = draw_nand2(
        lib,
        &GateParams {
            name: format!("{name}_nand"),
            size: Size {
                nmos_width: 1_200,
                pmos_width: 1_200,
            },
            length: 150,
        },
    )?;

    let inv = draw_inv(
        lib,
        &GateParams {
            name: format!("{name}_inv"),
            size: Size {
                nmos_width: 1200,
                pmos_width: 1200,
            },
            length: 150,
        },
    )?;

    let mut inv = Instance::new("inv", inv);
    inv.reflect_vert_anchored();
    let mut nand1 = Instance::new("nand_forward", nand.clone());
    let mut nand2 = Instance::new("nand_out", nand);

    let inv_bbox = inv.bbox();
    nand1.align_beneath(inv_bbox, 200);
    let nand1_bbox = nand1.bbox();
    nand2.align_beneath(nand1_bbox, 200);
    let nand2_bbox = nand2.bbox();

    let mut router = Router::new(format!("{name}_route"), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    let src = inv.port("din_b").largest_rect(m0).unwrap();
    let dst2 = nand2.port("b").largest_rect(m0).unwrap();
    let dst1 = nand1.port("b").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .vert_to(nand1_bbox.p1.y)
        .horiz_to(dst1.left())
        .up()
        .vert_to(dst2.bottom())
        .contact_down(dst2)
        .contact_down(dst1);

    // Join VDD
    let width = 3 * cfg.line(1);

    for (inv_port, nand_port, stack) in [("vdd", "vdd", "ntap"), ("vss", "vss", "ptap")] {
        let dst0 = inv.port(inv_port).largest_rect(m0).unwrap();
        let dst1 = nand1.port(nand_port).largest_rect(m0).unwrap();
        let dst2 = nand2.port(nand_port).largest_rect(m0).unwrap();

        let tap = lib
            .pdk
            .get_contact_sized(stack, Dir::Horiz, m0, dst0.width())
            .unwrap();
        let mut top_tap = Instance::new(format!("{inv_port}_tap_top"), tap.cell.clone());
        top_tap.align_above(inv_bbox, 200);
        top_tap.align_centers_horizontally_gridded(dst2.into(), cfg.grid());
        let dst3 = top_tap.port("x").largest_rect(m0).unwrap();

        let mut bot_tap = Instance::new(format!("{inv_port}_tap_bot"), tap.cell.clone());
        bot_tap.align_beneath(nand2_bbox, 200);
        bot_tap.align_centers_horizontally_gridded(dst2.into(), cfg.grid());
        let dst4 = bot_tap.port("x").largest_rect(m0).unwrap();

        let xspan = Span::from_center_span_gridded(dst2.center().x, width, cfg.grid());
        let span = Span::new(dst4.bottom() - 100, dst3.top() + 100);
        let rect = Rect::span_builder()
            .with(Dir::Horiz, xspan)
            .with(Dir::Vert, span)
            .build();

        let mut trace = router.trace(rect, 1);
        trace
            .contact_down(dst0)
            .contact_down(dst1)
            .contact_down(dst2)
            .contact_down(dst3)
            .contact_down(dst4);

        let mut port = AbstractPort::new(nand_port.to_lowercase());
        port.add_shape(m1, Shape::Rect(rect));
        abs.add_port(port);

        layout.add_inst(top_tap);
        layout.add_inst(bot_tap);
    }

    abs.add_port(inv.port("din").named("clk_in"));
    abs.add_port(nand1.port("a").named("din"));
    abs.add_port(nand1.port("y").named("clk_out"));
    abs.add_port(nand2.port("a").named("en"));
    abs.add_port(nand2.port("y").named("dout"));

    layout.add_inst(inv);
    layout.add_inst(nand1);
    layout.add_inst(nand2);
    layout.add_inst(router.finish());

    let ptr = Ptr::new(Cell {
        name: name.to_string(),
        layout: Some(layout),
        abs: Some(abs),
    });
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

/// A single delay unit (one forward cell and `multiplier-1` backwards cells).
pub fn draw_tmc_unit(lib: &mut PdkLib, params: &TmcUnitParams) -> Result<Ptr<Cell>> {
    assert!(params.multiplier >= 2);

    let delay_cell = draw_dbdr_delay_cell(lib, &format!("{}_delay_cell", &params.name))?;
    let mut router = Router::new(format!("{}_route", &params.name), lib.pdk.clone());

    let mut cell = Cell::empty(&params.name);

    let fwd = Instance::new("forwards", delay_cell.clone());

    let fwd_bbox = fwd.bbox();
    let mut bbox = fwd_bbox;

    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: Some(pdkprims::bus::ContactPosition::CenteredNonAdjacent),
            below: Some(pdkprims::bus::ContactPosition::CenteredNonAdjacent),
        },
    );

    // allocate space for VDD and forward/backward connections
    let cell_spacing = 2 * space + 3 * cfg.line(1);

    let mut backwards_cells = Vec::with_capacity(params.multiplier - 1);
    for i in 0..(params.multiplier - 1) {
        let mut backwards = Instance::new(format!("backwards_{i}"), delay_cell.clone());
        backwards.align_to_the_right_of(bbox, cell_spacing);
        bbox = backwards.bbox();
        backwards_cells.push(backwards.clone());
        cell.layout_mut().add_inst(backwards);
    }

    for i in 0..(params.multiplier - 2) {
        let src = backwards_cells[i].port("clk_out").largest_rect(m0).unwrap();
        let dst = backwards_cells[i + 1]
            .port("clk_in")
            .largest_rect(m0)
            .unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor_centered()
            .horiz_to(src.p1.x + cfg.line(0) + cfg.space(0) + 50)
            .vert_to(dst.p1.y)
            .horiz_to(dst.p1.x);
    }

    let mut leftmost_rect = None;
    let mut vdd_port_num = 0;
    let mut vss_port_num = 0;

    #[allow(clippy::explicit_counter_loop)]
    for inst in std::iter::once(&fwd).chain(backwards_cells.iter()) {
        let dst = inst.port("din").largest_rect(m0).unwrap();
        let mut rect = Rect::span_builder()
            .with(Dir::Vert, bbox.into_rect().vspan())
            .with(
                Dir::Horiz,
                Span::new(
                    dst.left() - 3 * cfg.line(1) - 2 * cfg.space(1),
                    dst.left() - 2 * cfg.space(1),
                ),
            )
            .build();

        if inst.inst_name != "backwards_0" {
            let vdd = router.trace(rect, 1);
            cell.add_pin(format!("vdd{vdd_port_num}"), m1, rect);
            vdd_port_num += 1;
            let mut trace = router.trace(dst, 0);
            trace
                .place_cursor(Dir::Horiz, false)
                .horiz_to_trace(&vdd)
                .contact_up(vdd.rect());
        } else {
            let dst = inst.port("clk_in").largest_rect(m0).unwrap();
            rect.p1.y = dst.p1.y + 100;
            let clk_in = router.trace(rect, 1);
            cell.add_pin("clk_rev", m1, rect);

            let mut trace = router.trace(dst, 0);
            trace
                .place_cursor(Dir::Horiz, false)
                .horiz_to_trace(&clk_in)
                .contact_up(clk_in.rect());
        }

        if inst.inst_name == "forwards" {
            // Enable
            let rect = Rect::span_builder()
                .with(Dir::Vert, bbox.into_rect().vspan())
                .with(
                    Dir::Horiz,
                    Span::new(
                        rect.left() - 3 * cfg.line(1) - 2 * cfg.space(1),
                        rect.left() - 2 * cfg.space(1),
                    ),
                )
                .build();
            let en = router.trace(rect, 1);
            cell.add_pin("sae_in", m1, rect);
            let dst = inst.port("en").largest_rect(m0).unwrap();
            let mut trace = router.trace(dst, 0);
            trace
                .place_cursor(Dir::Horiz, false)
                .horiz_to_trace(&en)
                .contact_up(en.rect());
            leftmost_rect = Some(rect);
        }

        cell.add_pin_from_port(inst.port("vdd").named(format!("vdd{vdd_port_num}")), m1);
        vdd_port_num += 1;
        cell.add_pin_from_port(inst.port("vss").named(format!("vss{vss_port_num}")), m1);
        vss_port_num += 1;
    }

    // clk_in
    cell.add_pin_from_port(fwd.port("clk_in"), m0);

    // sae_out
    let last = backwards_cells.last().unwrap();
    cell.add_pin_from_port(last.port("clk_out").named("sae_out"), m0);

    let src = fwd.port("dout").largest_rect(m0).unwrap();
    let dst = backwards_cells[0].port("din").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .horiz_to(src.p1.x + cfg.line(0) + cfg.space(0) + 50)
        .vert_to(dst.p1.y)
        .horiz_to(dst.p1.x);

    let src = fwd.port("clk_out").largest_rect(m0).unwrap();
    let rect = leftmost_rect.unwrap();
    let mut rect = Rect::span_builder()
        .with(Dir::Vert, bbox.into_rect().vspan())
        .with(
            Dir::Horiz,
            Span::new(
                rect.left() - 3 * cfg.line(1) - 2 * cfg.space(1),
                rect.left() - 2 * cfg.space(1),
            ),
        )
        .build();
    rect.p1.y = src.p1.y + 200;
    let clk_out = router.trace(rect, 1);

    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .up()
        .up()
        .horiz_to(rect.left())
        .contact_down(clk_out.rect());

    cell.add_pin("clk_out", m1, clk_out.rect());
    cell.layout_mut().add_inst(router.finish());
    cell.layout_mut().add_inst(fwd);

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_tmc(lib: &mut PdkLib, params: &TmcParams) -> Result<Ptr<Cell>> {
    assert!(params.multiplier >= 2);

    let delay_unit = draw_tmc_unit(
        lib,
        &TmcUnitParams {
            name: format!("{}_delay_unit", &params.name),
            multiplier: params.multiplier,
        },
    )?;

    let mut cell = Cell::empty(&params.name);

    let mut cells: Vec<Instance> = Vec::with_capacity(params.units);
    for i in 0..params.units {
        let mut inst = Instance::new(format!("delay_{i}"), delay_unit.clone());
        if i > 0 {
            inst.align_beneath(cells[i - 1].bbox(), 500);
        }
        cells.push(inst.clone());
        cell.layout_mut().add_inst(inst);
    }

    let mut router = Router::new(format!("{}_route", &params.name), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    for i in 0..(params.units - 1) {
        let src = cells[i].port("clk_out").largest_rect(m1).unwrap();
        let dst = cells[i + 1].port("clk_in").largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 1);
        trace
            .set_width(src.width())
            .place_cursor_centered()
            .vert_to(dst.p0.y - 100);
        router
            .trace(dst, 0)
            .place_cursor_centered()
            .horiz_to_trace(&trace)
            .contact_up(trace.rect());

        let src = cells[i + 1].port("sae_out").largest_rect(m0).unwrap();
        let dst = cells[i].port("clk_rev").largest_rect(m1).unwrap();

        let mut trace = router.trace(src, 0);
        trace
            .place_cursor_centered()
            .up()
            .vert_to(dst.p0.y + 250)
            .up()
            .horiz_to(dst.p0.x)
            .contact_down(dst);
    }

    ///////////////////////////////
    // Merge ports
    ///////////////////////////////
    let top = &cells[0];
    let bot = &cells[cells.len() - 1];

    for port in ["sae_in".to_string()]
        .into_iter()
        .chain((0..(2 * params.multiplier - 1)).map(|i| format!("vdd{i}")))
        .chain((0..(params.multiplier)).map(|i| format!("vss{i}")))
    {
        let t = top.port(&port).largest_rect(m1).unwrap();
        let b = bot.port(&port).largest_rect(m1).unwrap();
        let rect = Rect::new(b.p0, t.p1);
        router.trace(rect, 1);
        cell.add_pin(port, m1, rect);
    }

    // Connect clk_rev of the last delay unit to vss1
    let src = bot.port("clk_rev").largest_rect(m1).unwrap();
    // vss1 is to the right of clk_rev
    let dst = bot.port("vss1").largest_rect(m1).unwrap();
    let mut trace = router.trace(src, 1);
    trace
        .place_cursor_centered()
        .up()
        .horiz_to(dst.p1.x)
        .contact_down(dst);

    cell.add_pin_from_port(cells[0].port("sae_out"), m0);

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
