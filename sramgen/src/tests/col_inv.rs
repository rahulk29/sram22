use crate::layout::col_inv::*;
use crate::schematic::col_inv::*;
use crate::tests::test_gds_path;
use crate::utils::save_modules;
use crate::{generate_netlist, Result};
use pdkprims::tech::sky130;

#[test]
fn test_col_inv_array() -> Result<()> {
    let name = "sramgen_col_inv_array";
    let width = 32;
    let modules = col_inv_array(ColInvArrayParams {
        name: name.to_string(),
        width,
        instance_params: ColInvParams {
            length: 150,
            nwidth: 1_400,
            pwidth: 2_600,
        },
    });
    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_col_inv_array(&mut lib, name, width as usize, 2)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
