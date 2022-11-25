use crate::layout::inv_chain::*;
use crate::paths::out_gds;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_inv_chain_grid() -> Result<()> {
    let name = "sramgen_inv_chain_grid_5x9";

    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_chain_grid(
        &mut lib,
        InvChainGridParams {
            prefix: name,
            rows: 5,
            cols: 9,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

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

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}
