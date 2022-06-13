use layout21::{raw::Cell, utils::Ptr};
use pdkprims::{geometry::CoarseDirection, PdkLib};

use super::array::*;
use crate::{tech::sramgen_sp_sense_amp_gds, Result};

pub fn draw_sense_amp_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    let sa = sramgen_sp_sense_amp_gds(lib)?;

    {
        let sa = sa.read().unwrap();
        let lay = sa.layout.as_ref().unwrap();
        println!("sa bbox: {:?}", lay.bbox());
    }

    draw_cell_array(
        ArrayCellParams {
            name: "sense_amp_array".to_string(),
            num: width,
            cell: sa,
            spacing: Some(2 * 2500),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: CoarseDirection::Horizontal,
        },
        lib,
    )
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;

    #[test]
    fn test_sky130_sense_amp_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_sense_amp_array")?;
        draw_sense_amp_array(&mut lib, 16)?;

        lib.save_gds()?;

        Ok(())
    }
}
