use crate::gate::{GateParams, Size};
use crate::layout::Result;
use crate::tech::COLUMN_WIDTH;
use layout21::raw::geom::Dir;
use layout21::raw::{Cell, Instance};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::array::{draw_cell_array, ArrayCellParams, FlipMode};
use super::bank::GateList;
use super::common::{MergeArgs, NWELL_COL_SIDE_EXTEND, NWELL_COL_VERT_EXTEND};

pub fn draw_col_inv_array(lib: &mut PdkLib, prefix: &str, width: usize) -> Result<Ptr<Cell>> {
    let cell = draw_col_inv(lib, &format!("{prefix}_cell"))?;

    let array = draw_cell_array(
        ArrayCellParams {
            name: format!("{}_array_inst", prefix),
            num: width,
            cell,
            spacing: Some(COLUMN_WIDTH * 2),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let inst = Instance::new("array", array.cell);

    let mut cell = Cell::empty(prefix);
    for port in inst.ports() {
        cell.abs_mut().add_port(port);
    }

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();
    let elt = MergeArgs::builder()
        .layer(nwell)
        .insts(GateList::Array(&inst, width))
        .port_name("vpb")
        .left_overhang(NWELL_COL_SIDE_EXTEND)
        .right_overhang(NWELL_COL_SIDE_EXTEND)
        .build()?
        .element();
    cell.layout_mut().add(elt);

    cell.layout_mut().add_inst(inst);

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

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_col_inv_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_col_inv_array")?;
        draw_col_inv_array(&mut lib, "test_col_inv_array", 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
