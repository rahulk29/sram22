use crate::config::ControlMode;
use crate::layout::common::MergeArgs;
use crate::layout::inv_chain::{draw_inv_chain_grid, InvChainGridParams};
use crate::layout::sram::GateList;
use crate::tech::{sc_and2_gds, sc_buf_gds, sc_bufbuf_16_gds, sc_inv_gds, sc_nor2_gds, sc_tap_gds};
use crate::Result;

use layout21::raw::{Cell, Instance, Point};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::common::sc_outline;
use super::inv_chain::{draw_inv_chain, InvChainParams};
use super::route::Router;

pub fn draw_control_logic(lib: &mut PdkLib, mode: ControlMode) -> Result<Ptr<Cell>> {
    assert_eq!(mode, ControlMode::Simple);
    let mut cell = Cell::empty("sram22_control_logic");

    let and = sc_and2_gds(lib)?;
    let inv = sc_inv_gds(lib)?;
    let buf = sc_buf_gds(lib)?;
    let tap = sc_tap_gds(lib)?;
    let delay_chain = draw_inv_chain_grid(
        lib,
        InvChainGridParams {
            prefix: "sram22_control_logic_delay_chain",
            rows: 5,
            cols: 9,
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
    delay_chain.loc.y = 5 * inv_outline.height();
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

    for port in delay_chain.ports_starting_with("vdd") {
        let name = format!("{}_chain", &port.net);
        cell.add_pin_from_port(port.named(name), m1);
    }
    for port in delay_chain.ports_starting_with("vss") {
        let name = format!("{}_chain", &port.net);
        cell.add_pin_from_port(port.named(name), m1);
    }

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

    cell.abs_mut().add_port(delay_chain.port("m2_block"));

    cell.layout_mut().add_inst(inv);
    cell.layout_mut().add_inst(buf);
    cell.layout_mut().add_inst(and);
    cell.layout_mut().add_inst(delay_chain);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_control_logic_replica_v1(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty("sramgen_control_replica_v1");

    let and = sc_and2_gds(lib)?;
    let inv = sc_inv_gds(lib)?;
    let buf = sc_bufbuf_16_gds(lib)?;
    let tap = sc_tap_gds(lib)?;
    let nor2 = sc_nor2_gds(lib)?;

    // edge detector delay chain
    let eddc = draw_inv_chain(
        lib,
        InvChainParams {
            prefix: "sram22_control_logic_edge_detector_delay_chain",
            num: 7,
        },
    )?;
    // SAE set delay chain
    let ssdc = draw_inv_chain(
        lib,
        InvChainParams {
            prefix: "sram22_control_logic_sae_delay_chain",
            num: 4,
        },
    )?;
    // precharge delay chain
    let pcdc = draw_inv_chain(
        lib,
        InvChainParams {
            prefix: "sram22_control_logic_pc_delay_chain",
            num: 16,
        },
    )?;

    let mut x = 0;
    let mut y = 0;
    let eddc = Instance::new("delay_chain", eddc);
    let eddc_outline = sc_outline(&lib.pdk, &eddc);
    x += eddc_outline.width();
    let mut ed_and = Instance::new("edge_detector_and", and.clone());
    let and_outline = sc_outline(&lib.pdk, &ed_and);
    ed_and.loc.x = x;
    x += and_outline.width();
    let mut tap0 = Instance::new("tap0", tap.clone());
    let tap_outline = sc_outline(&lib.pdk, &tap0);
    tap0.loc.x = x;
    y += tap_outline.height();
    x = 0;

    let mut tap1 = Instance::new("tap1", tap.clone());
    tap1.loc = Point::new(x, y);
    tap1.reflect_vert_anchored();
    x += tap_outline.width();
    let mut inv_rbl = Instance::new("inv_rbl", inv.clone());
    let inv_outline = sc_outline(&lib.pdk, &inv_rbl);
    inv_rbl.loc = Point::new(x, y);
    inv_rbl.reflect_vert_anchored();
    x += inv_outline.width();
    let mut wl_ctl_nor1 = Instance::new("wl_ctl_nor1", nor2.clone());
    let nor2_outline = sc_outline(&lib.pdk, &wl_ctl_nor1);
    wl_ctl_nor1.loc = Point::new(x, y);
    wl_ctl_nor1.reflect_vert_anchored();
    x += nor2_outline.width();
    let mut wl_ctl_nor2 = Instance::new("wl_ctl_nor2", nor2.clone());
    wl_ctl_nor2.loc = Point::new(x, y);
    wl_ctl_nor2.reflect_vert_anchored();
    x += nor2_outline.width();
    let mut wl_en_buf = Instance::new("wl_en_buf", buf.clone());
    let buf_outline = sc_outline(&lib.pdk, &wl_en_buf);
    wl_en_buf.loc = Point::new(x, y);
    wl_en_buf.reflect_vert_anchored();
    x += buf_outline.width();
    let mut tap2 = Instance::new("tap2", tap.clone());
    tap2.loc = Point::new(x, y);
    tap2.reflect_vert_anchored();

    y += tap_outline.height();
    x = 0;
    let mut ssdc_inst = Instance::new("sae_delay_chain", ssdc.clone());
    let ssdc_outline = sc_outline(&lib.pdk, &ssdc_inst);
    ssdc_inst.loc = Point::new(x, y);
    x += ssdc_outline.width();
    let mut sae_ctl_nor1 = Instance::new("sae_ctl_nor1", nor2.clone());
    sae_ctl_nor1.loc = Point::new(x, y);
    x += nor2_outline.width();
    let mut sae_ctl_nor2 = Instance::new("sae_ctl_nor2", nor2.clone());
    sae_ctl_nor2.loc = Point::new(x, y);
    x += nor2_outline.width();
    let mut sae_buf = Instance::new("sae_buf", buf.clone());
    sae_buf.loc = Point::new(x, y);
    x += buf_outline.width();
    let mut tap3 = Instance::new("tap3", tap.clone());
    tap3.loc = Point::new(x, y);

    y += tap_outline.height();
    x = 0;

    let mut pcdc = Instance::new("pc_delay_chain", pcdc);
    pcdc.loc = Point::new(x, y);
    pcdc.reflect_vert_anchored();

    y += tap_outline.height();

    let mut tap4 = Instance::new("tap4", tap.clone());
    tap4.loc = Point::new(x, y);
    x += tap_outline.width();
    let mut pc_ctl_nor1 = Instance::new("pc_ctl_nor1", nor2.clone());
    pc_ctl_nor1.loc = Point::new(x, y);
    x += nor2_outline.width();
    let mut pc_ctl_nor2 = Instance::new("pc_ctl_nor2", nor2.clone());
    pc_ctl_nor2.loc = Point::new(x, y);
    x += nor2_outline.width();
    let mut pc_b_buf = Instance::new("pc_b_buf", buf.clone());
    pc_b_buf.loc = Point::new(x, y);
    x += buf_outline.width();
    let mut tap5 = Instance::new("tap5", tap.clone());
    tap5.loc = Point::new(x, y);

    x = 0;
    y += tap_outline.height();

    let mut tap6 = Instance::new("tap6", tap.clone());
    tap6.loc = Point::new(x, y);
    tap6.reflect_vert_anchored();
    x += tap_outline.width();
    let mut and_wr_en_set = Instance::new("and_wr_en_set", and.clone());
    and_wr_en_set.loc = Point::new(x, y);
    and_wr_en_set.reflect_vert_anchored();
    x += nor2_outline.width();
    let mut wr_drv_dc = Instance::new("wr_drv_set_delay_chain", ssdc);
    wr_drv_dc.loc = Point::new(x, y);
    x += ssdc_outline.width();
    wr_drv_dc.reflect_vert_anchored();
    let mut wr_drv_ctl_nor1 = Instance::new("wr_drv_ctl_nor1", nor2.clone());
    wr_drv_ctl_nor1.loc = Point::new(x, y);
    wr_drv_ctl_nor1.reflect_vert_anchored();
    x += nor2_outline.width();
    let mut wr_drv_ctl_nor2 = Instance::new("wr_drv_ctl_nor2", nor2.clone());
    wr_drv_ctl_nor2.loc = Point::new(x, y);
    x += nor2_outline.width();
    wr_drv_ctl_nor2.reflect_vert_anchored();
    let mut wr_drv_buf = Instance::new("wr_drv_buf", buf.clone());
    wr_drv_buf.loc = Point::new(x, y);
    wr_drv_buf.reflect_vert_anchored();
    x += buf_outline.width();
    let mut tap7 = Instance::new("tap7", tap.clone());
    tap7.loc = Point::new(x, y);
    tap7.reflect_vert_anchored();

    let mut router = Router::new("sram22_control_logic_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    cell.layout_mut().add_inst(eddc);
    cell.layout_mut().add_inst(ed_and);
    cell.layout_mut().add_inst(tap0);
    cell.layout_mut().add_inst(tap1);
    cell.layout_mut().add_inst(inv_rbl);
    cell.layout_mut().add_inst(wl_ctl_nor1);
    cell.layout_mut().add_inst(wl_ctl_nor2);
    cell.layout_mut().add_inst(wl_en_buf);
    cell.layout_mut().add_inst(tap2);
    cell.layout_mut().add_inst(ssdc_inst);
    cell.layout_mut().add_inst(sae_ctl_nor1);
    cell.layout_mut().add_inst(sae_ctl_nor2);
    cell.layout_mut().add_inst(sae_buf);
    cell.layout_mut().add_inst(tap3);
    cell.layout_mut().add_inst(pcdc);
    cell.layout_mut().add_inst(tap4);
    cell.layout_mut().add_inst(pc_ctl_nor1);
    cell.layout_mut().add_inst(pc_ctl_nor2);
    cell.layout_mut().add_inst(pc_b_buf);
    cell.layout_mut().add_inst(tap5);
    cell.layout_mut().add_inst(tap6);
    cell.layout_mut().add_inst(and_wr_en_set);
    cell.layout_mut().add_inst(wr_drv_dc);
    cell.layout_mut().add_inst(wr_drv_ctl_nor1);
    cell.layout_mut().add_inst(wr_drv_ctl_nor2);
    cell.layout_mut().add_inst(wr_drv_buf);
    cell.layout_mut().add_inst(tap7);

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
