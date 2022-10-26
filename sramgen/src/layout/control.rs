use crate::layout::bank::GateList;
use crate::layout::common::MergeArgs;
use crate::tech::{sc_and2_gds, sc_inv_gds, sc_tap_gds, sc_buf_gds};
use crate::Result;

use layout21::raw::{Cell, Instance};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::common::sc_outline;
use super::route::Router;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum ControlMode {
    Simple,
    SimpleChipSelect,
    Replica,
}

pub struct InvChainParams<'a> {
    prefix: &'a str,
    num: usize,
}

pub fn draw_inv_chain(lib: &mut PdkLib, params: InvChainParams) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty(params.prefix);

    let inv = sc_inv_gds(lib)?;
    let tap = sc_tap_gds(lib)?;

    let tap0 = Instance::new("tap0", tap.clone());
    let tmp = Instance::new("", inv.clone());
    let inv_outline = sc_outline(&lib.pdk, &tmp);
    let tap_outline = sc_outline(&lib.pdk, &tap0);

    let mut router = Router::new(format!("{}_route", params.prefix), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    let mut x = tap_outline.p1.x;
    let mut prev: Option<Instance> = None;
    for i in 0..params.num {
        let mut inv = Instance::new(format!("inv_{}", i), inv.clone());
        inv.loc.x = x;
        x += inv_outline.width();
        if let Some(prev) = prev {
            let dst = prev.port("y").largest_rect(m0).unwrap();
            let src = inv.port("a").largest_rect(m0).unwrap();

            let mut trace = router.trace(src, 0);
            trace
                .place_cursor(layout21::raw::Dir::Horiz, false)
                .horiz_to(dst.left());
        }
        cell.layout_mut().add_inst(inv.clone());

        if i == 0 {
            let rect = inv.port("a").largest_rect(m0).unwrap();
            cell.add_pin("din", m0, rect);
        } else if i == params.num - 1 {
            let rect = inv.port("y").largest_rect(m0).unwrap();
            cell.add_pin("dout", m0, rect);
        }

        prev = Some(inv);
    }

    let mut tap1 = Instance::new("tap1", tap);
    tap1.loc.x = x;

    cell.layout_mut().add_inst(tap0);
    cell.layout_mut().add_inst(tap1);

    let rect = MergeArgs::builder()
        .layer(m1)
        .insts(GateList::Cells(&cell.layout().insts))
        .port_name("vgnd")
        .build()?
        .rect();
    cell.add_pin("vss", m1, rect);

    let rect = MergeArgs::builder()
        .layer(m1)
        .insts(GateList::Cells(&cell.layout().insts))
        .port_name("vpwr")
        .build()?
        .rect();
    cell.add_pin("vdd", m1, rect);

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_control_logic(lib: &mut PdkLib, mode: ControlMode) -> Result<Ptr<Cell>> {
    assert_eq!(mode, ControlMode::Simple);
    let mut cell = Cell::empty("sram22_control_logic");

    let and = sc_and2_gds(lib)?;
    let inv = sc_inv_gds(lib)?;
    let buf = sc_buf_gds(lib)?;
    let tap = sc_tap_gds(lib)?;
    let delay_chain = draw_inv_chain(lib, InvChainParams { prefix: "sram22_control_logic_delay_chain", num: 25 })?;

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

    let mut router = Router::new("sram22_control_logic_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    // pc_b to buffer
    let src = inv.port("y").largest_rect(m0).unwrap();
    let dst = buf.port("x").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace.place_cursor_centered().vert_to(src.center().y - 225).horiz_to_rect(dst);
    cell.add_pin("pc_b", m0, trace.rect());

    // buffer to and gate
    let src = buf.port("a").largest_rect(m0).unwrap();
    let dst = and.port("a").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace.place_cursor_centered().up().horiz_to_rect(dst).contact_down(dst);
    cell.add_pin("wl_en", m1, trace.rect());
    cell.add_pin_from_port(and.port("x").named("write_driver_en"), m0);

    // connect clocks
    let src = inv.port("a").largest_rect(m0).unwrap();
    let dst = delay_chain.port("din").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace.place_cursor_centered().up().up().vert_to_rect(dst).contact_down(dst);
    cell.add_pin("clk", m2, trace.rect());
    cell.add_pin_from_port(delay_chain.port("dout").named("sense_en"), m0);

    cell.layout_mut().add_inst(tap0);
    cell.layout_mut().add_inst(inv);
    cell.layout_mut().add_inst(buf);
    cell.layout_mut().add_inst(and);
    cell.layout_mut().add_inst(tap1);
    cell.layout_mut().add_inst(delay_chain);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_control_logic_simple() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_control_logic_simple")?;
        draw_control_logic(&mut lib, ControlMode::Simple)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_inv_chain_12() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_inv_chain_12")?;
        draw_inv_chain(
            &mut lib,
            InvChainParams {
                prefix: "test_sky130_inv_chain_12",
                num: 12,
            },
        )?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
