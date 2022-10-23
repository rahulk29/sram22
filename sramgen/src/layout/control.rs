use crate::layout::bank::GateList;
use crate::layout::common::MergeArgs;
use crate::tech::{sc_and2_gds, sc_inv_gds, sc_tap_gds};
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
    assert_eq!(params.num % 2, 0);
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
    let tap = sc_tap_gds(lib)?;

    let tap0 = Instance::new("tap0", tap.clone());
    let mut tap1 = Instance::new("tap1", tap);
    let mut and = Instance::new("and0", and);
    let and_outline = sc_outline(&lib.pdk, &and);
    let tap_outline = sc_outline(&lib.pdk, &tap0);

    and.loc.x = tap_outline.width();
    tap1.loc.x = tap_outline.width() + and_outline.width();

    cell.layout_mut().add_inst(tap0);
    cell.layout_mut().add_inst(and);
    cell.layout_mut().add_inst(tap1);

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
