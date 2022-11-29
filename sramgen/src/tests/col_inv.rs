use crate::config::col_inv::*;
use crate::layout::col_inv::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::col_inv::*;
use crate::schematic::{generate_netlist, save_modules};
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_col_inv_array() -> Result<()> {
    let name = "sramgen_col_inv_array";
    let params = ColInvArrayParams {
        name: name.to_string(),
        width: 32,
        mux_ratio: 2,
        instance_params: ColInvParams {
            name: "col_inv".to_string(),
            length: 150,
            nwidth: 1_400,
            pwidth: 2_600,
        },
    };

    let modules = col_inv_array(&params);

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_col_inv_array(&mut lib, &params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
