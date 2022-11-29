use crate::config::inv_chain::{InvChainGridParams, InvChainParams};
use crate::config::sram::ControlMode;
use crate::layout::common::MergeArgs;
use crate::layout::inv_chain::draw_inv_chain_grid;
use crate::layout::sram::GateList;
use crate::tech::{sc_and2_gds, sc_buf_gds, sc_bufbuf_16_gds, sc_inv_gds, sc_nor2_gds, sc_tap_gds};
use crate::Result;

use layout21::raw::{BoundBoxTrait, Cell, Dir, Instance, Point, Rect};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::common::sc_outline;
use super::inv_chain::draw_inv_chain;
use super::power::{PowerSource, PowerStrapGen, PowerStrapOpts};
use super::route::Router;

pub fn draw_control_logic(lib: &mut PdkLib, mode: ControlMode) -> Result<Ptr<Cell>> {
    match mode {
        ControlMode::Simple => draw_control_logic_simple(lib),
        ControlMode::ReplicaV1 => draw_control_logic_replica_v1(lib),
    }
}

pub fn draw_control_logic_simple(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty("sram22_control_logic");

    let and = sc_and2_gds(lib)?;
    let inv = sc_inv_gds(lib)?;
    let buf = sc_buf_gds(lib)?;
    let tap = sc_tap_gds(lib)?;
    let delay_chain = draw_inv_chain_grid(
        lib,
        &InvChainGridParams {
            name: "sram22_control_logic_delay_chain".to_string(),
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
        &InvChainParams {
            name: "sram22_control_logic_edge_detector_delay_chain".to_string(),
            num: 7,
        },
    )?;
    // SAE set delay chain
    let ssdc = draw_inv_chain(
        lib,
        &InvChainParams {
            name: "sram22_control_logic_sae_delay_chain".to_string(),
            num: 4,
        },
    )?;
    // precharge delay chain
    let pcdc = draw_inv_chain(
        lib,
        &InvChainParams {
            name: "sram22_control_logic_pc_delay_chain".to_string(),
            num: 16,
        },
    )?;

    // Place standard cells
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
    let mut inv_rbl = Instance::new("inv_rbl", inv);
    let inv_outline = sc_outline(&lib.pdk, &inv_rbl);
    inv_rbl.loc = Point::new(x, y);
    inv_rbl.reflect_vert_anchored();
    x += inv_outline.width();
    let mut wl_ctl_nor1 = Instance::new("wl_ctl_nor1", nor2.clone());
    let nor2_outline = sc_outline(&lib.pdk, &wl_ctl_nor1);
    wl_ctl_nor1.loc = Point::new(x, y);
    wl_ctl_nor1.reflect_vert_anchored();
    wl_ctl_nor1.reflect_horiz_anchored();
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
    sae_ctl_nor1.reflect_horiz_anchored();
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
    pc_ctl_nor1.reflect_horiz_anchored();
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
    let mut and_wr_en_set = Instance::new("and_wr_en_set", and);
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
    wr_drv_ctl_nor1.reflect_horiz_anchored();
    x += nor2_outline.width();
    let mut wr_drv_ctl_nor2 = Instance::new("wr_drv_ctl_nor2", nor2);
    wr_drv_ctl_nor2.loc = Point::new(x, y);
    x += nor2_outline.width();
    wr_drv_ctl_nor2.reflect_vert_anchored();
    let mut wr_drv_buf = Instance::new("wr_drv_buf", buf);
    wr_drv_buf.loc = Point::new(x, y);
    wr_drv_buf.reflect_vert_anchored();
    x += buf_outline.width();
    let mut tap7 = Instance::new("tap7", tap);
    tap7.loc = Point::new(x, y);
    tap7.reflect_vert_anchored();

    // Routing
    let mut router = Router::new("sram22_control_logic_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    // Edge detector
    let clk_in = eddc.port("din").largest_rect(m0).unwrap();
    let clk_out = eddc.port("dout").largest_rect(m0).unwrap();
    let and_a = ed_and.port("a").largest_rect(m0).unwrap();
    let and_b = ed_and.port("b").largest_rect(m0).unwrap();

    let mut trace = router.trace(and_a, 0);
    trace
        .place_cursor_centered()
        .up()
        .horiz_to_rect(clk_in)
        .contact_down(clk_in);

    cell.add_pin("clk", m1, trace.rect());
    let mut trace = router.trace(and_b, 0);
    trace
        .place_cursor_centered()
        .up()
        .down_by(3 * cfg.line(0))
        .horiz_to_rect(clk_out)
        .contact_down(clk_out);

    // Wordline control latch (wl_ctl)
    let (_, wl_en0) = route_latch(&wl_ctl_nor1, &wl_ctl_nor2, &mut router, true);

    let buf_a = wl_en_buf.port("a").largest_rect(m0).unwrap();
    router
        .trace(wl_en0, 1)
        .set_width(cfg.line(0))
        .place_cursor(Dir::Horiz, true)
        .horiz_to(buf_a.right() - 140)
        .set_width(230)
        .vert_to_rect(buf_a)
        .contact_down(buf_a);

    cell.add_pin_from_port(inv_rbl.port("a").named("rbl"), m0);
    let rst = wl_ctl_nor1.port("a").largest_rect(m0).unwrap();
    let rbl_b = inv_rbl.port("y").largest_rect(m0).unwrap();
    router
        .trace(rst, 0)
        .place_cursor_centered()
        .up()
        .horiz_to_rect(rbl_b)
        .contact_down(rbl_b);

    cell.add_pin_from_port(wl_en_buf.port("x").named("wl_en"), m1);

    // TODO clkp -> wl latch set

    let ssdc_din = ssdc_inst.port("din").largest_rect(m0).unwrap();
    router
        .trace(rbl_b, 0)
        .place_cursor_centered()
        .up()
        .up()
        .vert_to(ssdc_din.top() - 200)
        .down()
        .horiz_to_rect(ssdc_din)
        .contact_down(ssdc_din);

    let (sense_en0, _) = route_latch(&sae_ctl_nor1, &sae_ctl_nor2, &mut router, true);
    let sense_en_set = sae_ctl_nor1.port("a").largest_rect(m0).unwrap();
    let ssdc_set = ssdc_inst.port("dout").largest_rect(m0).unwrap();
    let mut trace = router.trace(sense_en_set, 0);
    trace
        .place_cursor_centered()
        .up()
        .horiz_to_rect(ssdc_set)
        .contact_down(ssdc_set);
    let sense_en_set = trace.rect();

    let buf_a = sae_buf.port("a").largest_rect(m0).unwrap();
    router
        .trace(sense_en0, 1)
        .set_width(cfg.line(0))
        .place_cursor(Dir::Horiz, true)
        .horiz_to(buf_a.right() - 140)
        .set_width(230)
        .vert_to_rect(buf_a)
        .contact_down(buf_a);

    cell.add_pin_from_port(sae_buf.port("x").named("sense_en"), m1);

    // Generate precharge bar
    let pcdc_din = pcdc.port("din").largest_rect(m0).unwrap();
    router
        .trace(sense_en_set, 1)
        .place_cursor_centered()
        .up()
        .vert_to(pcdc_din.top() - 200)
        .down()
        .horiz_to_rect(pcdc_din)
        .contact_down(pcdc_din);
    let pcdc_dout = pcdc.port("dout").largest_rect(m0).unwrap();
    let pc_set = pc_ctl_nor2.port("a").largest_rect(m0).unwrap();
    router
        .trace(pcdc_dout, 0)
        .place_cursor_centered()
        .up()
        .left_by(3 * cfg.space(0))
        .up_by(4 * cfg.space(0))
        .horiz_to_rect(pc_set)
        .up()
        .vert_to_rect(pc_set)
        .down()
        .down();
    // pc_set -> pc_ctl_nor1

    let (pc_b0, _) = route_latch(&pc_ctl_nor1, &pc_ctl_nor2, &mut router, true);
    let buf_a = pc_b_buf.port("a").largest_rect(m0).unwrap();
    router
        .trace(pc_b0, 1)
        .set_width(cfg.line(0))
        .place_cursor(Dir::Horiz, true)
        .horiz_to(buf_a.right() - 140)
        .set_width(230)
        .vert_to_rect(buf_a)
        .contact_down(buf_a);

    cell.add_pin_from_port(pc_b_buf.port("x").named("pc_b"), m1);

    // Write driver control
    cell.add_pin_from_port(and_wr_en_set.port("b").named("we"), m0);
    let wr_drv_dc_in = wr_drv_dc.port("din").largest_rect(m0).unwrap();
    let wr_drv_set0 = and_wr_en_set.port("x").largest_rect(m0).unwrap();
    router
        .trace(wr_drv_set0, 0)
        .place_cursor_centered()
        .up()
        .horiz_to(wr_drv_dc_in.right() - 140)
        .vert_to_rect(wr_drv_dc_in)
        .contact_down(wr_drv_dc_in);

    // TODO sense_en0 -> wr_drv_ctl_nor2
    let wr_drv_reset = wr_drv_ctl_nor2.port("a").largest_rect(m0).unwrap();
    router
        .trace(sense_en0, 1)
        .set_width(cfg.line(0))
        .place_cursor(Dir::Horiz, true)
        .horiz_to_rect(wr_drv_reset)
        .up()
        .vert_to_rect(wr_drv_reset)
        .down()
        .down();

    let wr_drv_dc_dout = wr_drv_dc.port("dout").largest_rect(m0).unwrap();
    let wr_en_set = wr_drv_ctl_nor1.port("a").largest_rect(m0).unwrap();
    router
        .trace(wr_en_set, 0)
        .place_cursor_centered()
        .up()
        .horiz_to_rect(wr_drv_dc_dout)
        .contact_down(wr_drv_dc_dout);

    let (write_driver_en0, _) = route_latch(&wr_drv_ctl_nor1, &wr_drv_ctl_nor2, &mut router, false);
    let buf_a = wr_drv_buf.port("a").largest_rect(m0).unwrap();
    router
        .trace(write_driver_en0, 1)
        .set_width(cfg.line(0))
        .place_cursor(Dir::Horiz, true)
        .horiz_to(buf_a.right() - 140)
        .set_width(230)
        .vert_to_rect(buf_a)
        .contact_down(buf_a);

    cell.add_pin_from_port(wr_drv_buf.port("x").named("write_driver_en"), m1);

    let clkp_out = ed_and.port("x").largest_rect(m0).unwrap();
    let wl_ctl_clkp = wl_ctl_nor2.port("a").largest_rect(m0).unwrap();
    let sae_ctl_clkp = sae_ctl_nor2.port("a").largest_rect(m0).unwrap();
    let pc_ctl_clkp = pc_ctl_nor1.port("a").largest_rect(m0).unwrap();
    let wr_set_clkp = and_wr_en_set.port("a").largest_rect(m0).unwrap();

    let mut clkp_trace = router.trace(clkp_out, 0);
    clkp_trace
        .place_cursor_centered()
        .up()
        .horiz_to_rect(wl_ctl_clkp)
        .up()
        .vert_to_rect(wl_ctl_clkp);

    router
        .trace(wl_ctl_clkp, 0)
        .contact_up(clkp_trace.rect())
        .increment_layer()
        .contact_up(clkp_trace.rect());

    clkp_trace
        .down()
        .horiz_to_rect(sae_ctl_clkp)
        .up()
        .vert_to_rect(sae_ctl_clkp)
        .down()
        .down()
        .increment_layer()
        .increment_layer()
        .up_by(2_000)
        .down()
        .horiz_to_rect(pc_ctl_clkp)
        .up()
        .vert_to_rect(pc_ctl_clkp)
        .down()
        .down()
        .increment_layer()
        .increment_layer()
        .vert_to(wr_set_clkp.top() - 200)
        .down()
        .down();

    let vss_rows = [
        vec![eddc.clone(), ed_and.clone(), tap0.clone()],
        vec![
            ssdc_inst.clone(),
            sae_ctl_nor1.clone(),
            sae_ctl_nor2.clone(),
            sae_buf.clone(),
            tap3.clone(),
        ],
        vec![pcdc.clone()],
        vec![
            tap6.clone(),
            and_wr_en_set.clone(),
            wr_drv_dc.clone(),
            wr_drv_ctl_nor1.clone(),
            wr_drv_ctl_nor2.clone(),
            wr_drv_buf.clone(),
            tap7.clone(),
        ],
    ];

    let vdd_rows = [
        vec![
            tap1.clone(),
            inv_rbl.clone(),
            wl_ctl_nor1.clone(),
            wl_ctl_nor2.clone(),
            wl_en_buf.clone(),
            tap2.clone(),
        ],
        vec![
            ssdc_inst.clone(),
            sae_ctl_nor1.clone(),
            sae_ctl_nor2.clone(),
            sae_buf.clone(),
            tap3.clone(),
        ],
        vec![
            tap6.clone(),
            and_wr_en_set.clone(),
            wr_drv_dc.clone(),
            wr_drv_ctl_nor1.clone(),
            wr_drv_ctl_nor2.clone(),
            wr_drv_buf.clone(),
            tap7.clone(),
        ],
    ];

    let mut vss_rects = vec![];
    for row in vss_rows {
        let rect = MergeArgs::builder()
            .layer(m1)
            .insts(GateList::Cells(&row))
            .port_name("vgnd")
            .build()?
            .rect();
        vss_rects.push(rect);
    }
    let mut vdd_rects = vec![];
    for row in vdd_rows {
        let rect = MergeArgs::builder()
            .layer(m1)
            .insts(GateList::Cells(&row))
            .port_name("vpwr")
            .build()?
            .rect();
        vdd_rects.push(rect);
    }

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

    let mut power_grid = PowerStrapGen::new(
        &PowerStrapOpts::builder()
            .h_metal(2)
            .h_line(10)
            .h_space(10)
            .v_metal(2)
            .v_line(640)
            .v_space(3 * cfg.space(2))
            .pdk(lib.pdk.clone())
            .name("sramgen_control_replica_v1_power_straps")
            .enclosure(
                cell.layout()
                    .bbox()
                    .into_rect()
                    .expand_dir(Dir::Horiz, -4 * 640),
            )
            .omit_dir(Dir::Horiz)
            .build()?,
    );
    for rect in vss_rects {
        power_grid.add_gnd_target(1, rect);
    }
    for rect in vdd_rects {
        power_grid.add_vdd_target(1, rect);
    }

    let route = router.finish();
    {
        let route = route.cell.read().unwrap();
        for elem in route.layout().elems.iter() {
            if elem.layer == m2 {
                power_grid.add_padded_blockage(2, elem.inner.bbox().into_rect().expand(75));
            }
        }
    }

    cell.layout_mut().add_inst(route);

    let straps = power_grid.generate()?;
    for (src, rect) in straps.v_traces {
        let net = match src {
            PowerSource::Vdd => "vdd",
            PowerSource::Gnd => "vss",
        };
        cell.add_pin(net, m2, rect);
    }
    cell.layout_mut().add_inst(straps.instance);

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

fn route_latch(
    nor1: &Instance,
    nor2: &Instance,
    router: &mut Router,
    invert_routing: bool,
) -> (Rect, Rect) {
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);

    let b1 = nor1.port("b").largest_rect(m0).unwrap();
    let b2 = nor2.port("b").largest_rect(m0).unwrap();
    let q = nor1.port("y").largest_rect(m0).unwrap();
    let qb = nor2.port("y").largest_rect(m0).unwrap();

    let mut trace = router.trace(b1, 0);
    trace.place_cursor_centered().up();
    if invert_routing {
        trace.up_by(2 * cfg.line(0));
    } else {
        trace.down_by(2 * cfg.line(0));
    }
    trace.horiz_to_rect(qb).contact_down(qb);
    let qout = trace.rect();
    let mut trace = router.trace(b2, 0);
    trace.place_cursor_centered().up();
    if invert_routing {
        trace.down_by(2 * cfg.line(0));
    } else {
        trace.up_by(2 * cfg.line(0));
    }
    trace.horiz_to_rect(q).contact_down(q);
    let qbout = trace.rect();

    (qout, qbout)
}
