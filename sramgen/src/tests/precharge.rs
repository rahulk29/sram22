use vlsir::circuit::Package;

use crate::layout::precharge::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::precharge::*;
use crate::schematic::{generate_netlist, save_bin};
use crate::tech::all_external_modules;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_precharge() -> Result<()> {
    let name = "sramgen_precharge";
    let params = PrechargeParams {
        name: name.to_string(),
        length: 150,
        pull_up_width: 1_200,
        equalizer_width: 1_000,
    };
    let pc = precharge(params.clone());
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: "sramgen_precharge".to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![pc],
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_precharge(&mut lib, params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_precharge_array() -> Result<()> {
    let name = "sramgen_precharge_array";
    let params = PrechargeArrayParams {
        width: 32,
        instance_params: PrechargeParams {
            name: "sramgen_precharge".to_string(),
            length: 150,
            pull_up_width: 1_200,
            equalizer_width: 1_000,
        },
        name: name.to_string(),
    };
    let modules = precharge_array(params.clone());
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules,
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_precharge_array(&mut lib, params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
