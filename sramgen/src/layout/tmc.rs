//! Timing multiplier circuit layout.

use anyhow::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Dir, Instance, Layout, Rect, Shape, Span,
};
use layout21::utils::Ptr;
use pdkprims::bus::ContactPolicy;
use pdkprims::PdkLib;

use crate::gate::{GateParams, Size};

use super::gate::{draw_inv, draw_nand2};
use super::route::Router;

pub fn draw_dbdr_delay_cell(lib: &mut PdkLib, name: &str) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);

    let nand = draw_nand2(
        lib,
        GateParams {
            name: format!("{}_nand", name),
            size: Size {
                nmos_width: 1_200,
                pmos_width: 1_200,
            },
            length: 150,
        },
    )?;

    let inv = draw_inv(
        lib,
        GateParams {
            name: format!("{}_inv", name),
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

    let mut router = Router::new(format!("{}_route", name), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    let src = inv.port("din_b").largest_rect(m0).unwrap();
    let dst2 = nand2.port("B").largest_rect(m0).unwrap();
    let dst1 = nand1.port("B").largest_rect(m0).unwrap();
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

    for (inv_port, nand_port, stack) in [("vdd", "VDD", "ntap"), ("gnd", "VSS", "ptap")] {
        let dst0 = inv.port(inv_port).largest_rect(m0).unwrap();
        let dst1 = nand1.port(nand_port).largest_rect(m0).unwrap();
        let dst2 = nand2.port(nand_port).largest_rect(m0).unwrap();

        let tap = lib
            .pdk
            .get_contact_sized(stack, Dir::Horiz, m0, dst0.width())
            .unwrap();
        let mut top_tap = Instance::new(format!("{}_tap_top", inv_port), tap.cell.clone());
        top_tap.align_above(inv_bbox, 200);
        top_tap.align_centers_horizontally_gridded(dst2.into(), cfg.grid());
        let dst3 = top_tap.port("x").largest_rect(m0).unwrap();

        let mut bot_tap = Instance::new(format!("{}_tap_bot", inv_port), tap.cell.clone());
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
    abs.add_port(nand1.port("A").named("din"));
    abs.add_port(nand1.port("Y").named("clk_out"));
    abs.add_port(nand2.port("A").named("en"));
    abs.add_port(nand2.port("Y").named("dout"));

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

pub struct TMCUnitParams {
    /// The name of the timing multiplier circuit cell.
    name: String,
    /// The timing multiplier (must be at least 2).
    multiplier: usize,
}

/// A single delay unit (one forward cell and `multiplier-1` backwards cells).
pub fn draw_tmc_unit(lib: &mut PdkLib, params: TMCUnitParams) -> Result<Ptr<Cell>> {
    assert!(params.multiplier >= 2);
    let mut layout = Layout::new(&params.name);
    let abs = Abstract::new(&params.name);

    let delay_cell = draw_dbdr_delay_cell(lib, &format!("{}_delay_cell", &params.name))?;
    let fwd = Instance::new("forwards", delay_cell.clone());

    let fwd_bbox = fwd.bbox();
    let mut bbox = fwd_bbox;

    let mut router = Router::new(format!("{}_route", &params.name), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: Some(pdkprims::bus::ContactPosition::CenteredNonAdjacent),
            below: Some(pdkprims::bus::ContactPosition::CenteredNonAdjacent),
        },
    );
    // allocate space for VDD and forward/backward connections
    let cell_spacing = 4 * space + 3 * cfg.line(1);

    let mut backwards_cells = Vec::with_capacity(params.multiplier - 1);
    for i in 0..(params.multiplier - 1) {
        let mut backwards = Instance::new(format!("backwards_{i}"), delay_cell.clone());
        backwards.align_to_the_right_of(bbox, cell_spacing);
        bbox = backwards.bbox();
        backwards_cells.push(backwards.clone());
        layout.add_inst(backwards);
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

    for cell in std::iter::once(&fwd).chain(backwards_cells.iter()) {
        let dst = cell.port("din").largest_rect(m0).unwrap();
        let rect = Rect::span_builder()
            .with(Dir::Vert, bbox.into_rect().vspan())
            .with(
                Dir::Horiz,
                Span::new(
                    dst.left() - 3 * cfg.line(1) - 2 * cfg.space(1),
                    dst.left() - 2 * cfg.space(1),
                ),
            )
            .build();
        let vdd = router.trace(rect, 1);
        if cell.inst_name != "backwards_0" {
            let mut trace = router.trace(dst, 0);
            trace
                .place_cursor(Dir::Horiz, false)
                .horiz_to_trace(&vdd)
                .contact_up(vdd.rect());
        }

        if cell.inst_name == "forwards" {
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
            let dst = cell.port("en").largest_rect(m0).unwrap();
            let mut trace = router.trace(dst, 0);
            trace
                .place_cursor(Dir::Horiz, false)
                .horiz_to_trace(&en)
                .contact_up(en.rect());
            leftmost_rect = Some(rect);
        }
    }

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
    let clk_fwd = router.trace(rect, 1);

    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .up()
        .up()
        .horiz_to(rect.left())
        .contact_down(clk_fwd.rect());
    layout.add_inst(router.finish());

    layout.add_inst(fwd);

    let ptr = Ptr::new(Cell {
        name: params.name,
        layout: Some(layout),
        abs: Some(abs),
    });
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_dbdr_delay_cell() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_dbdr_delay_cell")?;
        draw_dbdr_delay_cell(&mut lib, "test_sky130_dbdr_delay_cell")?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_tmc_unit_6() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_tmc_unit_6")?;
        draw_tmc_unit(
            &mut lib,
            TMCUnitParams {
                name: "test_sky130_tmc_unit_6".to_string(),
                multiplier: 6,
            },
        )?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
