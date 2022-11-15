use vlsir::circuit::Package;

use crate::layout::precharge::*;
use crate::save_bin;
use crate::schematic::precharge::*;
use crate::tech::all_external_modules;
use crate::tests::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_precharge() -> Result<()> {
    let params = PrechargeParams {
        name: "test_sky130_precharge".to_string(),
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

    save_bin("precharge", pkg)?;
    let mut lib = sky130::pdk_lib("test_sky130_precharge")?;
    draw_precharge(&mut lib, params)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_precharge_array() -> Result<()> {
    let params = PrechargeArrayParams {
        width: 32,
        instance_params: PrechargeParams {
            name: "precharge".to_string(),
            length: 150,
            pull_up_width: 1_200,
            equalizer_width: 1_000,
        },
        name: "precharge_array".to_string(),
    };
    let modules = precharge_array(params.clone());
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: "sramgen_precharge_array".to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules,
        ext_modules,
    };

    save_bin("precharge_array", pkg)?;

    let mut lib = sky130::pdk_lib("test_sky130_precharge_array")?;
    draw_precharge_array(&mut lib, params)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
