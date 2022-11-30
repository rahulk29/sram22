use layout21::raw::{AbstractPort, BoundBoxTrait, Cell, Dir, Instance, Point, Rect};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use crate::config::inv_chain::{InvChainGridParams, InvChainParams};
use crate::config::sram::ControlMode;
use crate::layout::common::{sc_outline, MergeArgs};
use crate::layout::inv_chain::{draw_inv_chain, draw_inv_chain_grid};
use crate::layout::power::{PowerSource, PowerStrapGen, PowerStrapOpts};
use crate::layout::route::Router;
use crate::layout::rows::AlignedRows;
use crate::layout::sram::GateList;
use crate::tech::{sc_and2_gds, sc_buf_gds, sc_bufbuf_16_gds, sc_inv_gds, sc_nor2_gds, sc_tap_gds};
use crate::Result;

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
    let delay_chain_7 = draw_inv_chain(
        lib,
        &InvChainParams {
            name: "sram22_control_logic_edge_detector_delay_chain".to_string(),
            num: 7,
        },
    )?;
    // SAE set delay chain
    let delay_chain_4 = draw_inv_chain(
        lib,
        &InvChainParams {
            name: "sram22_control_logic_delay_chain_4".to_string(),
            num: 4,
        },
    )?;
    // precharge delay chain
    let delay_chain_16 = draw_inv_chain(
        lib,
        &InvChainParams {
            name: "sram22_control_logic_delay_chain_16".to_string(),
            num: 16,
        },
    )?;

    let mut rows = AlignedRows::new();
    // Place standard cells
    rows.add_row(vec![
        Instance {
            inst_name: "delay_chain".to_string(),
            cell: delay_chain_7.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "edge_detector_and".to_string(),
            cell: and.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "tap0".to_string(),
            cell: tap.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
    ]);

    let eddc = rows.get(0, 0);
    let ed_and = rows.get(0, 1);
    let tap0 = rows.get(0, 2);

    rows.add_row(vec![
        Instance {
            inst_name: "tap1".to_string(),
            cell: tap.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "inv_rbl".to_string(),
            cell: inv.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "wl_ctl_nor1".to_string(),
            cell: nor2.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "wl_ctl_nor2".to_string(),
            cell: nor2.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "wl_en_buf".to_string(),
            cell: buf.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "tap2".to_string(),
            cell: tap.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
    ]);

    let tap1 = rows.get(1, 0);
    let inv_rbl = rows.get(1, 1);
    let wl_ctl_nor1 = rows.get(1, 2);
    let wl_ctl_nor2 = rows.get(1, 3);
    let wl_en_buf = rows.get(1, 4);
    let tap2 = rows.get(1, 5);

    rows.add_row(vec![
        Instance {
            inst_name: "wl_drv_delay_chain".to_string(),
            cell: delay_chain_4.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "inv_we".to_string(),
            cell: inv,
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "cond1".to_string(),
            cell: and.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "wr_en_detector_delay_chain".to_string(),
            cell: delay_chain_7,
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "wr_en_detector_and".to_string(),
            cell: and.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "cond2".to_string(),
            cell: and.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
    ]);

    let wl_set_dc0 = rows.get(2, 0);
    let inv_we = rows.get(2, 1);
    let cond1 = rows.get(2, 2);
    let wdeddc = rows.get(2, 3);
    let wded_and = rows.get(2, 4);
    let cond2 = rows.get(2, 5);

    rows.add_row(vec![
        Instance {
            inst_name: "sae_delay_chain".to_string(),
            cell: delay_chain_4.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "sae_ctl_nor1".to_string(),
            cell: nor2.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "sae_ctl_nor2".to_string(),
            cell: nor2.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "sae_buf".to_string(),
            cell: buf.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "tap3".to_string(),
            cell: tap.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
    ]);

    let ssdc_inst = rows.get(3, 0);
    let sae_ctl_nor1 = rows.get(3, 1);
    let sae_ctl_nor2 = rows.get(3, 2);
    let sae_buf = rows.get(3, 3);
    let tap3 = rows.get(3, 4);

    rows.add_row(vec![Instance {
        inst_name: "pc_delay_chain".to_string(),
        cell: delay_chain_16,
        loc: Point::new(0, 0),
        reflect_vert: true,
        angle: None,
    }]);

    let pcdc = rows.get(4, 0);

    rows.add_row(vec![
        Instance {
            inst_name: "tap4".to_string(),
            cell: tap.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "pc_ctl_nor1".to_string(),
            cell: nor2.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "pc_ctl_nor2".to_string(),
            cell: nor2.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "pc_b_buf".to_string(),
            cell: buf.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
        Instance {
            inst_name: "tap5".to_string(),
            cell: tap.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
    ]);

    let tap4 = rows.get(5, 0);
    let pc_ctl_nor1 = rows.get(5, 1);
    let pc_ctl_nor2 = rows.get(5, 2);
    let pc_b_buf = rows.get(5, 3);
    let tap5 = rows.get(5, 4);

    rows.add_row(vec![
        Instance {
            inst_name: "tap6".to_string(),
            cell: tap.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "and_wr_en_set".to_string(),
            cell: and,
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "wr_drv_set_delay_chain".to_string(),
            cell: delay_chain_4,
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "wr_drv_ctl_nor1".to_string(),
            cell: nor2.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "wr_drv_ctl_nor2".to_string(),
            cell: nor2,
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "wr_drv_buf".to_string(),
            cell: buf,
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
        Instance {
            inst_name: "tap7".to_string(),
            cell: tap,
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
    ]);

    let tap6 = rows.get(6, 0);
    let and_wr_en_set = rows.get(6, 1);
    let wr_drv_dc = rows.get(6, 2);
    let wr_drv_ctl_nor1 = rows.get(6, 3);
    let wr_drv_ctl_nor2 = rows.get(6, 4);
    let wr_drv_buf = rows.get(6, 5);
    let tap7 = rows.get(6, 6);

    rows.place(&lib.pdk);

    /*
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
        cell.add_pin("vgnd", m1, rect);
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
        cell.add_pin("vpwr", m1, rect);
    }
    */

    cell.layout_mut().insts = rows.into_instances();

    /*
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

    let mut port = AbstractPort::new("m2_block");
    let route = router.finish();
    {
        let route = route.cell.read().unwrap();
        for elem in route.layout().elems.iter() {
            if elem.layer == m2 {
                let rect = elem.inner.bbox().into_rect().expand(75);
                power_grid.add_padded_blockage(2, rect);
                port.add_shape(m2, layout21::raw::Shape::Rect(rect));
            }
        }
    }
    cell.abs_mut().add_port(port);

    cell.layout_mut().add_inst(route);

    if false {
        let straps = power_grid.generate()?;
        for (src, rect) in straps.v_traces {
            let net = match src {
                PowerSource::Vdd => "vdd",
                PowerSource::Gnd => "vss",
            };
            cell.add_pin(net, m2, rect);
        }
        cell.layout_mut().add_inst(straps.instance);
    }
    */

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
