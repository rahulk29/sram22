use crate::config::ControlMode;
use crate::layout::common::MergeArgs;
use crate::layout::inv_chain::{draw_inv_chain, InvChainParams};
use crate::layout::sram::GateList;
use crate::tech::{sc_and2_gds, sc_buf_gds, sc_inv_gds, sc_tap_gds};
use crate::Result;

use layout21::raw::{Cell, Instance};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::common::sc_outline;
use super::route::Router;

pub fn draw_control_logic(lib: &mut PdkLib, mode: ControlMode) -> Result<Ptr<Cell>> {
    assert_eq!(mode, ControlMode::Simple);
    let mut cell = Cell::empty("sram22_control_logic");

    let and = sc_and2_gds(lib)?;
    let inv = sc_inv_gds(lib)?;
    let buf = sc_buf_gds(lib)?;
    let tap = sc_tap_gds(lib)?;
    let delay_chain = draw_inv_chain(
        lib,
        InvChainParams {
            prefix: "sram22_control_logic_delay_chain",
            num: 25,
        },
    )?;

    let tap0 = Instance::new("tap0", tap.clone());
    let mut tap1 = Instance::new("tap1", tap);
    let mut inv = Instance::new("inv0", inv);
    let mut buf = Instance::new("buf0", buf);
    let mut and = Instance::new("and0", and);
    let mut delay_chain = Instance::new("delay_chain", delay_chain);
    let inv_outline = sc_outline(&lib.pdk, &inv);
    let and_outline = sc_outline(&lib.pdk, &and);
    let tap_outline = sc_outline(&lib.pdk, &tap0);
    let buf_outline = sc_outline(&lib.pdk, &buf);

    inv.loc.x = tap_outline.width();
    buf.loc.x = inv.loc.x + inv_outline.width();
    and.loc.x = buf.loc.x + buf_outline.width();
    tap1.loc.x = and.loc.x + and_outline.width();
    delay_chain.loc.y = inv_outline.height();
    delay_chain.reflect_vert_anchored();
    buf.reflect_horiz_anchored();

    let mut router = Router::new("sram22_control_logic_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    // pc_b to buffer
    let src = inv.port("y").largest_rect(m0).unwrap();
    let dst = buf.port("a").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .vert_to(dst.center().y - 170 / 2)
        .up()
        .horiz_to_rect(dst)
        .contact_down(dst);
    cell.add_pin("pc_b", m1, trace.rect());

    // buffer to and gate
    let src = buf.port("x").largest_rect(m0).unwrap();
    let dst = and.port("a").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .up()
        .horiz_to_rect(dst)
        .contact_down(dst);
    cell.add_pin("wl_en", m1, trace.rect());
    cell.add_pin_from_port(and.port("b").named("we"), m0);
    cell.add_pin_from_port(and.port("x").named("write_driver_en"), m0);

    // connect clocks
    let src = inv.port("a").largest_rect(m0).unwrap();
    let dst = delay_chain.port("din").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor_centered()
        .up()
        .up()
        .vert_to_rect(dst)
        .contact_down(dst)
        .decrement_layer()
        .contact_down(dst);
    cell.add_pin("clk", m2, trace.rect());
    cell.add_pin_from_port(delay_chain.port("dout").named("sense_en"), m0);

    cell.add_pin_from_port(delay_chain.port("vdd").named("vdd0"), m1);
    cell.add_pin_from_port(delay_chain.port("vss").named("vss0"), m1);

    cell.layout_mut().add_inst(tap0);
    cell.layout_mut().add_inst(tap1);

    let port = MergeArgs::builder()
        .layer(m1)
        .insts(GateList::Cells(&cell.layout().insts))
        .port_name("vgnd")
        .build()?
        .port()
        .named("vss1");
    cell.add_pin_from_port(port, m1);

    cell.layout_mut().add_inst(inv);
    cell.layout_mut().add_inst(buf);
    cell.layout_mut().add_inst(and);
    cell.layout_mut().add_inst(delay_chain);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
