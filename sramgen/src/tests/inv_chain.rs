use crate::layout::inv_chain::*;
use crate::schematic::inv_chain::inv_chain_grid;
use crate::tech::all_external_modules;
use crate::tests::test_gds_path;
use crate::{generate_netlist, save_bin, Result};
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_inv_chain_grid() -> Result<()> {
    let name = "sramgen_inv_chain_grid_5x9";

    let params = InvChainGridParams {
        prefix: name,
        rows: 5,
        cols: 9,
    };
    let inv_chain = inv_chain_grid(params);
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![inv_chain],
        ext_modules,
    };

    save_bin(name, pkg)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_chain_grid(&mut lib, params)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_inv_chain_12() -> Result<()> {
    let name = "sramgen_inv_chain_12";
    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_chain(
        &mut lib,
        InvChainParams {
            prefix: name,
            num: 12,
        },
    )?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
