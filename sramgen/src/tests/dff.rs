use crate::config::dff::*;
use crate::layout::dff::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::dff::*;
use crate::schematic::{generate_netlist, save_bin};
use crate::tech::{all_external_modules, COLUMN_WIDTH};
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_dff_array() -> Result<()> {
    let name = "sramgen_dff_array";
    let width = 16;

    let dff_params = DffGridParams::builder()
        .name("wmask_dff_array")
        .cols(width)
        .rows(1)
        .build()?;
    let dffs = dff_grid(&dff_params);

    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: dffs,
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dff_grid(&mut lib, &dff_params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_dff_grid() -> Result<()> {
    let name = "sramgen_dff_grid";
    let rows = 4;
    let cols = 8;

    let params = DffGridParams::builder()
        .name(name)
        .rows(rows)
        .cols(cols)
        .row_pitch(4 * COLUMN_WIDTH)
        .build()?;
    let dffs = dff_grid(&params);

    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: dffs,
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dff_grid(&mut lib, &params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
