use crate::layout::wmask_control::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::gate::{AndParams, GateParams, Size};
use crate::schematic::wmask_control::*;
use crate::schematic::{generate_netlist, save_modules};
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_wmask_control_2() -> Result<()> {
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

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mask_control(&mut lib, params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
