use crate::config::bitcell_array::*;
use crate::layout::array::*;
use crate::layout::draw_bitcell;
use crate::paths::{out_bin, out_gds};
use crate::schematic::bitcell_array::*;
use crate::schematic::{generate_netlist, save_bin};
use crate::tech::all_external_modules;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_bitcell() -> Result<()> {
    let name = "sramgen_bitcell";
    let mut lib = sky130::pdk_lib(name)?;
    draw_bitcell(&mut lib)?;

    let work_dir = test_work_dir(name);

    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_bitcell_array_32x32() -> Result<()> {
    let name = "sramgen_bitcell_array_32x32";
    let rows = 32;
    let cols = 32;

    let bitcells = bitcell_array(BitcellArrayParams {
        rows,
        cols,
        dummy_rows: 2,
        dummy_cols: 2,
        name: name.to_string(),
    });
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![bitcells],
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_bitcell_array(rows, cols, 2, 2, &mut lib)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_bitcell_array_2x2() -> Result<()> {
    let name = "sramgen_bitcell_array_2x2";
    let rows = 2;
    let cols = 2;

    let bitcells = bitcell_array(BitcellArrayParams {
        rows,
        cols,
        dummy_rows: 2,
        dummy_cols: 2,
        name: name.to_string(),
    });
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![bitcells],
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_bitcell_array(rows, cols, 2, 2, &mut lib)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
