use pdkprims::tech::sky130;
use vlsir::circuit::Package;

use crate::config::inv_chain::*;
use crate::layout::inv_chain::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::inv_chain::*;
use crate::schematic::{generate_netlist, save_bin};
use crate::tech::all_external_modules;
use crate::tests::test_work_dir;
use crate::Result;

#[test]
fn test_inv_chain_grid() -> Result<()> {
    let name = "sramgen_inv_chain_grid_5x9";

    let params = InvChainGridParams {
        name: name.to_string(),
        rows: 5,
        cols: 9,
    };
    let inv_chain = inv_chain_grid(&params);
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![inv_chain.into()],
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_chain_grid(&mut lib, &params)?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_inv_chain_12() -> Result<()> {
    let name = "sramgen_inv_chain_12";
    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_chain(
        &mut lib,
        &InvChainParams {
            name: name.to_string(),
            num: 12,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}
