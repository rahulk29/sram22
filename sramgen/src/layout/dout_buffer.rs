use crate::config::gate::{GateParams, Size};
use crate::layout::Result;
use crate::tech::COLUMN_WIDTH;

use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{BoundBoxTrait, Cell, Instance};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::array::{draw_cell_array, ArrayCellParams, FlipMode};
use super::common::{draw_two_level_contact, MergeArgs, TwoLevelContactParams};
use super::route::Router;
use super::sram::{connect, ConnectArgs, GateList};

pub fn draw_dout_buffer_array(
    lib: &mut PdkLib,
    name: &str,
    width: usize,
    mux_ratio: usize,
) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty(name.to_string());

    let unit = draw_dout_buffer(lib, &format!("{name}_cell"))?;
    let array = draw_cell_array(
        ArrayCellParams {
            name: format!("{name}_array"),
            num: width,
            cell: unit,
            spacing: Some(COLUMN_WIDTH * mux_ratio as isize),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Vert,
        },
        lib,
    )?;

    let mut inst = Instance::new("dout_buffer_array", array.cell);
    inst.angle = Some(-90f64);

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();
    let rect = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Array(&inst, width))
        .port_name("vpb0")
        .left_overhang(0)
        .right_overhang(0)
        .build()?
        .rect();
    cell.layout_mut().draw_rect(nwell, rect);
    cell.add_pin("vpb0", nwell, rect);

    let rect = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Array(&inst, width))
        .port_name("vpb1")
        .left_overhang(0)
        .right_overhang(0)
        .build()?
        .rect();
    cell.layout_mut().draw_rect(nwell, rect);
    cell.add_pin("vpb1", nwell, rect);

    let mut router = Router::new(format!("{}_route", name), lib.pdk.clone());
    let cfg = router.cfg();
    let m2 = cfg.layerkey(2);

    for net in ["vss0", "vdd0", "vss1", "vdd1"] {
        let args = ConnectArgs::builder()
            .metal_idx(2)
            .port_idx(1)
            .router(&mut router)
            .port_name(net)
            .dir(Dir::Horiz)
            .insts(GateList::Array(&inst, width))
            .overhang(1_395)
            .build()?;
        let trace = connect(args);
        cell.add_pin(net, m2, trace.rect());
    }

    for port in inst.ports() {
        if port.net.starts_with("vpb") || port.net.starts_with("vdd") || port.net.starts_with("vss")
        {
            continue;
        }
        cell.abs_mut().add_port(port);
    }

    cell.layout_mut().add_inst(inst);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());
    Ok(ptr)
}

pub fn draw_dout_buffer(lib: &mut PdkLib, name: &str) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty(name.to_string());
    let inv1_cell = super::gate::draw_inv(
        lib,
        &GateParams {
            name: format!("{name}_inv1"),
            size: Size {
                nmos_width: 1_000,
                pmos_width: 1_600,
            },
            length: 150,
        },
    )?;
    let inv2_cell = super::gate::draw_inv(
        lib,
        &GateParams {
            name: format!("{name}_inv2"),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 3_200,
            },
            length: 150,
        },
    )?;

    let ptap1 = draw_ptap_cell(lib, 3)?;
    let ntap1 = draw_ntap_cell(lib, 4)?;
    let ptap2 = draw_ptap_cell(lib, 6)?;
    let ntap2 = draw_ntap_cell(lib, 8)?;

    let mut inv1 = Instance::new("inv1", inv1_cell.clone());
    inv1.reflect_vert_anchored();
    let inv1_bbox = inv1.bbox();
    let mut inv2 = Instance::new("inv2", inv2_cell.clone());
    inv2.align_to_the_right_of(inv1_bbox, 1_270);
    inv2.reflect_vert_anchored();
    let inv2_bbox = inv2.bbox();

    let mut inv1_d = Instance::new("inv1_dummy", inv1_cell);
    inv1_d.align_above(inv1_bbox, 1_000);
    let inv1_d_bbox = inv1_d.bbox();
    let mut inv2_d = Instance::new("inv2_dummy", inv2_cell);
    inv2_d.align_to_the_right_of(inv1_bbox, 1_270);
    inv2_d.align_centers_vertically_gridded(inv1_d_bbox, lib.pdk.grid());
    let inv2_d_bbox = inv2_d.bbox();

    let stage1_bbox = inv1_bbox.union(&inv1_d_bbox);
    let stage2_bbox = inv2_bbox.union(&inv2_d_bbox);

    let mut router = Router::new(format!("{name}_route"), lib.pdk.clone());
    let m0 = lib.pdk.metal(0);
    let m1 = lib.pdk.metal(1);

    let src = inv1.port("din_b").largest_rect(m0).unwrap();
    let dst = inv2.port("din").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace.s_bend(dst, Dir::Horiz);

    let src = inv1_d.port("din_b").largest_rect(m0).unwrap();
    let dst = inv2_d.port("din").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace.s_bend(dst, Dir::Horiz);

    // Place taps
    let nwell = lib.pdk.get_layerkey("nwell").unwrap();

    let nwell_region1 = inv1.port("vpb").largest_rect(nwell).unwrap();
    let mut pwell_region1 = inv1_bbox;
    pwell_region1.p1.x = nwell_region1.p0.x;
    let nwell_region2 = inv2.port("vpb").largest_rect(nwell).unwrap();
    let mut pwell_region2 = inv2_bbox;
    pwell_region2.p1.x = nwell_region2.p0.x;

    let mut ptap1 = Instance::new("ptap1", ptap1);
    ptap1.align_centers_horizontally_gridded(pwell_region1.bbox(), lib.pdk.grid());
    ptap1.align_centers_vertically_gridded(stage1_bbox, lib.pdk.grid());

    let mut ntap1 = Instance::new("ntap1", ntap1);
    ntap1.align_centers_horizontally_gridded(nwell_region1.bbox(), lib.pdk.grid());
    ntap1.align_centers_vertically_gridded(stage1_bbox, lib.pdk.grid());

    let mut ptap2 = Instance::new("ptap2", ptap2);
    ptap2.align_centers_horizontally_gridded(pwell_region2.bbox(), lib.pdk.grid());
    ptap2.align_centers_vertically_gridded(stage2_bbox, lib.pdk.grid());

    let mut ntap2 = Instance::new("ntap2", ntap2);
    ntap2.align_centers_horizontally_gridded(nwell_region2.bbox(), lib.pdk.grid());
    ntap2.align_centers_vertically_gridded(stage2_bbox, lib.pdk.grid());

    let src = ptap1.port("x").largest_rect(m0).unwrap();
    let dst = inv1.port("vss").largest_rect(m0).unwrap();
    let dst_d = inv1_d.port("vss").largest_rect(m0).unwrap();
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst);
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst_d);

    let src = ntap1.port("x").largest_rect(m0).unwrap();
    let dst = inv1.port("vdd").largest_rect(m0).unwrap();
    let dst_d = inv1_d.port("vdd").largest_rect(m0).unwrap();
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst);
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst_d);

    let src = ptap2.port("x").largest_rect(m0).unwrap();
    let dst = inv2.port("vss").largest_rect(m0).unwrap();
    let dst_d = inv2_d.port("vss").largest_rect(m0).unwrap();
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst);
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst_d);

    let src = ntap2.port("x").largest_rect(m0).unwrap();
    let dst = inv2.port("vdd").largest_rect(m0).unwrap();
    let dst_d = inv2_d.port("vdd").largest_rect(m0).unwrap();
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst);
    router
        .trace(src, 0)
        .place_cursor_centered()
        .vert_to_rect(dst_d);

    cell.add_pin_from_port(inv1.port("din").named("din1"), m0);
    cell.add_pin_from_port(inv2.port("din_b").named("dout1"), m0);

    cell.add_pin_from_port(inv1_d.port("din").named("din2"), m0);
    cell.add_pin_from_port(inv2_d.port("din_b").named("dout2"), m0);

    let rect = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Cells(&[inv1.clone(), inv1_d.clone()]))
        .port_name("vpb")
        .left_overhang(0)
        .right_overhang(0)
        .build()?
        .rect();
    cell.layout_mut().draw_rect(nwell, rect);
    cell.add_pin("vpb0", nwell, rect);
    let rect = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Cells(&[inv2.clone(), inv2_d.clone()]))
        .port_name("vpb")
        .left_overhang(0)
        .right_overhang(0)
        .build()?
        .rect();
    cell.layout_mut().draw_rect(nwell, rect);
    cell.add_pin("vpb1", nwell, rect);

    cell.add_pin_from_port(ptap1.port("x").named("vss0"), m1);
    cell.add_pin_from_port(ntap1.port("x").named("vdd0"), m1);
    cell.add_pin_from_port(ptap2.port("x").named("vss1"), m1);
    cell.add_pin_from_port(ntap2.port("x").named("vdd1"), m1);

    cell.layout_mut().add_inst(inv1);
    cell.layout_mut().add_inst(inv2);
    cell.layout_mut().add_inst(inv1_d);
    cell.layout_mut().add_inst(inv2_d);
    cell.layout_mut().add_inst(ptap1);
    cell.layout_mut().add_inst(ntap1);
    cell.layout_mut().add_inst(ptap2);
    cell.layout_mut().add_inst(ntap2);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());
    Ok(ptr)
}

fn draw_ntap_cell(lib: &mut PdkLib, cols: isize) -> Result<Ptr<Cell>> {
    let params = TwoLevelContactParams::builder()
        .name(format!("col_inv_ntap_cell_{cols}"))
        .bot_stack("ntap")
        .top_stack("viali")
        .bot_cols(cols)
        .top_cols(cols)
        .build()?;
    let contact = draw_two_level_contact(lib, params)?;
    Ok(contact)
}

fn draw_ptap_cell(lib: &mut PdkLib, cols: isize) -> Result<Ptr<Cell>> {
    let params = TwoLevelContactParams::builder()
        .name(format!("col_inv_ptap_cell_{cols}"))
        .bot_stack("ptap")
        .top_stack("viali")
        .bot_cols(cols)
        .top_cols(cols)
        .build()?;
    let contact = draw_two_level_contact(lib, params)?;
    Ok(contact)
}
