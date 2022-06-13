use crate::layout::Result;
use layout21::{raw::Cell, utils::Ptr};
use pdkprims::{geometry::CoarseDirection, PdkLib};

use super::array::{draw_cell_array, ArrayCellParams, FlipMode};

pub fn draw_nand2_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let nand2 = super::gate::draw_nand2(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "nand2_dec_array".to_string(),
            num: width,
            cell: nand2,
            spacing: Some(1580),
            flip: FlipMode::AlternateFlipVertical,
            flip_toggle: false,
            direction: CoarseDirection::Vertical,
        },
        lib,
    )
}

pub fn draw_inv_dec_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let inv_dec = super::gate::draw_inv_dec(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "inv_dec_array".to_string(),
            num: width,
            cell: inv_dec,
            spacing: Some(1580),
            flip: FlipMode::AlternateFlipVertical,
            flip_toggle: false,
            direction: CoarseDirection::Vertical,
        },
        lib,
    )
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;

    #[test]
    fn test_sky130_nand2_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_nand2_dec_array")?;
        draw_nand2_array(&mut lib, 32)?;

        lib.save_gds()?;

        Ok(())
    }

    #[test]
    fn test_sky130_inv_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_inv_dec_array")?;
        draw_inv_dec_array(&mut lib, 32)?;

        lib.save_gds()?;

        Ok(())
    }
}
