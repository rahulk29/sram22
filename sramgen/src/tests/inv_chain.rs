use crate::layout::inv_chain::*;
use crate::tests::test_gds_path;
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
