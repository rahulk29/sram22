use crate::bus_bit;
use crate::layout::Result;
use crate::schematic::gate::{GateParams, Size};
use crate::tech::COLUMN_WIDTH;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{BoundBoxTrait, Cell, Instance};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::array::{draw_cell_array, ArrayCellParams, FlipMode};
use super::bank::{connect, ConnectArgs, GateList};
use super::common::{draw_two_level_contact, MergeArgs, TwoLevelContactParams};

use super::route::Router;

pub fn draw_col_inv_array(
    lib: &mut PdkLib,
    prefix: &str,
    width: usize,
    mux_ratio: usize,
) -> Result<Ptr<Cell>> {
    let cell = draw_col_inv(lib, &format!("{prefix}_cell"))?;
    let ntap = draw_col_inv_ntap_cell(lib)?;
    let ptap = draw_col_inv_ptap_cell(lib)?;

    let mux_ratio = mux_ratio as isize;

    let array = draw_cell_array(
        ArrayCellParams {
            name: format!("{}_array_inst", prefix),
            num: width,
            cell,
            spacing: Some(COLUMN_WIDTH * mux_ratio),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let ntaps = draw_cell_array(
        ArrayCellParams {
            name: format!("{}_ntap_array", prefix),
            num: width + 1,
            cell: ntap,
            spacing: Some(COLUMN_WIDTH * mux_ratio),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let ptaps = draw_cell_array(
        ArrayCellParams {
            name: format!("{}_ptap_array", prefix),
            num: width + 1,
            cell: ptap,
            spacing: Some(COLUMN_WIDTH * mux_ratio),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let inst = Instance::new("array", array.cell);
    let mut ntaps = Instance::new("ntap_array", ntaps.cell);
    let mut ptaps = Instance::new("ptap_array", ptaps.cell);
    let inst_bbox = inst.bbox();
    ntaps.align_centers_horizontally_gridded(inst_bbox, lib.pdk.grid());
    ptaps.align_centers_horizontally_gridded(inst_bbox, lib.pdk.grid());

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();

    let nwell_region = inst.port(bus_bit("vpb", 0)).largest_rect(nwell).unwrap();
    let mut pwell_region = inst_bbox;
    pwell_region.p1.y = nwell_region.p0.y;

    ntaps.align_centers_vertically_gridded(nwell_region.bbox(), lib.pdk.grid());
    ptaps.align_centers_vertically_gridded(pwell_region.bbox(), lib.pdk.grid());

    let mut cell = Cell::empty(prefix);

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();
    let elt = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Array(&inst, width))
        .port_name("vpb")
        .left_overhang(mux_ratio * COLUMN_WIDTH / 2)
        .right_overhang(mux_ratio * COLUMN_WIDTH / 2)
        .build()?
        .element();
    cell.layout_mut().add(elt);

    let mut router = Router::new(format!("{}_route", &prefix), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m2 = cfg.layerkey(2);

    for i in 0..width {
        cell.add_pin_from_port(inst.port(bus_bit("din", i)), m0);
        cell.add_pin_from_port(inst.port(bus_bit("din_b", i)), m0);
    }

    let args = ConnectArgs::builder()
        .metal_idx(2)
        .port_idx(1)
        .router(&mut router)
        .port_name("x")
        .dir(Dir::Horiz)
        .insts(GateList::Array(&ntaps, width + 1))
        .overhang(100)
        .build()?;
    let trace = connect(args);
    cell.add_pin("vdd", m2, trace.rect());

    let args = ConnectArgs::builder()
        .metal_idx(2)
        .port_idx(1)
        .router(&mut router)
        .port_name("x")
        .dir(Dir::Horiz)
        .insts(GateList::Array(&ptaps, width + 1))
        .overhang(100)
        .build()?;
    let trace = connect(args);
    cell.add_pin("vss", m2, trace.rect());

    // Connect VDD/VSS to inverters
    for i in 0..width {
        for port in ["vdd", "vss"] {
            let src = if port == "vdd" { &ntaps } else { &ptaps };
            let tap_idx = if i % 2 == 0 { i } else { i + 1 };
            let src = src.port(bus_bit("x", tap_idx)).largest_rect(m0).unwrap();
            let dst = inst.port(bus_bit(port, i)).largest_rect(m0).unwrap();

            let mut trace = router.trace(src, 0);
            trace.place_cursor_centered().horiz_to(dst.center().x);
        }
    }

    cell.layout_mut().add_inst(inst);
    cell.layout_mut().add_inst(ntaps);
    cell.layout_mut().add_inst(ptaps);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_col_inv(lib: &mut PdkLib, name: &str) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty(name.to_string());
    let inv = super::gate::draw_inv(
        lib,
        GateParams {
            name: format!("{name}_inv"),
            size: Size {
                nmos_width: 1_400,
                pmos_width: 2_600,
            },
            length: 150,
        },
    )?;

    let mut inst = Instance::new("col_inv_inverter", inv);
    inst.angle = Some(90f64);

    for port in inst.ports() {
        cell.abs_mut().add_port(port);
    }
    cell.layout_mut().add_inst(inst);

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());
    Ok(ptr)
}

fn draw_col_inv_ntap_cell(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let params = TwoLevelContactParams::builder()
        .name("col_inv_ntap_cell")
        .bot_stack("ntap")
        .top_stack("viali")
        .bot_rows(6)
        .top_rows(6)
        .build()?;
    let contact = draw_two_level_contact(lib, params)?;
    Ok(contact)
}

fn draw_col_inv_ptap_cell(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let params = TwoLevelContactParams::builder()
        .name("col_inv_ptap_cell")
        .bot_stack("ptap")
        .top_stack("viali")
        .bot_rows(4)
        .top_rows(5)
        .build()?;
    let contact = draw_two_level_contact(lib, params)?;
    Ok(contact)
}
