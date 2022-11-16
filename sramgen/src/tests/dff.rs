use crate::layout::dff::*;
use crate::schematic::dff::*;
use crate::tech::{all_external_modules, COLUMN_WIDTH};
use crate::tests::test_gds_path;
use crate::{generate_netlist, save_bin, Result};
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_sky130_dff_array() -> Result<()> {
    let name = "sramgen_dff_array";
    let width = 16;

    let dffs = dff_array(DffArrayParams {
        width,
        name: name.to_string(),
    });

    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: dffs,
        ext_modules,
    };

    save_bin(name, pkg)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dff_array(&mut lib, name, width)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_dff_grid() -> Result<()> {
    let name = "sramgen_dff_grid";
    let rows = 4;
    let cols = 8;

    let dffs = dff_array(DffArrayParams {
        width: rows * cols,
        name: name.to_string(),
    });

    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: dffs,
        ext_modules,
    };

    save_bin(name, pkg)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    let params = DffGridParams::builder()
        .name(name)
        .rows(rows)
        .cols(cols)
        .row_pitch(4 * COLUMN_WIDTH)
        .build()?;
    draw_dff_grid(&mut lib, params)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
