use crate::{layout::Result, tech::openram_dff_gds};
use layout21::{
    raw::{Cell, Dir},
    utils::Ptr,
};
use pdkprims::PdkLib;

use crate::layout::array::*;

pub fn draw_dff_array(lib: &mut PdkLib, width: usize) -> Result<ArrayedCell> {
    let dff = openram_dff_gds(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "dff_array".to_string(),
            num: width,
            cell: dff,
            spacing: None,
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;
    #[test]
    fn test_sky130_dff_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_dff_array")?;
        draw_dff_array(&mut lib, 16)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
