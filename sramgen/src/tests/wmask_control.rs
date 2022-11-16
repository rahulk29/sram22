use crate::layout::wmask_control::*;
use crate::schematic::gate::{AndParams, GateParams, Size};
use crate::schematic::wmask_control::*;
use crate::tests::test_gds_path;
use crate::utils::save_modules;
use crate::{generate_netlist, Result};
use pdkprims::tech::sky130;

#[test]
fn test_sky130_wmask_control_2() -> Result<()> {
    let and_params = AndParams {
        name: "write_mask_control_and2".to_string(),
        nand: GateParams {
            name: "write_mask_control_and2_nand".to_string(),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 1_400,
            },
            length: 150,
        },
        inv: GateParams {
            name: "write_mask_control_and2_inv".to_string(),
            size: Size {
                nmos_width: 1_000,
                pmos_width: 1_400,
            },
            length: 150,
        },
    };

    let name = "sramgen_write_mask_control_2";
    let params = WriteMaskControlParams {
        name: name.to_string(),
        width: 2,
        and_params,
    };

    let modules = write_mask_control(params.clone());

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mask_control(&mut lib, params)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
