use crate::gate::{GateParams, Size};
use crate::layout::Result;
use layout21::raw::geom::Dir;
use layout21::raw::{Cell, Instance};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::array::{draw_cell_array, ArrayCellParams, ArrayedCell, FlipMode};

pub fn draw_col_inv_array(lib: &mut PdkLib, prefix: &str, width: usize) -> Result<ArrayedCell> {
    let cell = draw_col_inv(lib, &format!("{prefix}_cell"))?;

    draw_cell_array(
        ArrayCellParams {
            name: format!("{}_array", prefix),
            num: width,
            cell,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )
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
