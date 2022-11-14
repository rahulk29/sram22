use crate::layout::wmask_control::*;
use crate::schematic::gate::{AndParams, Size};
use crate::schematic::wmask_control::WriteMaskControlParams;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_wmask_control_2() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_wmask_control_2")?;
    draw_write_mask_control(
        &mut lib,
        WriteMaskControlParams {
            name: "wmask_control_2".to_string(),
            width: 2,
            and_params: AndParams {
                name: "wmask_control_and2".to_string(),
                nand_size: Size {
                    nmos_width: 2_000,
                    pmos_width: 1_400,
                },
                inv_size: Size {
                    nmos_width: 1_000,
                    pmos_width: 1_400,
                },
                length: 150,
            },
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
