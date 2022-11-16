use crate::config::ControlMode;
use crate::layout::control::*;
use crate::tests::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_control_logic_simple() -> Result<()> {
    let name = "sramgen_control_logic_simple";
    let mut lib = sky130::pdk_lib(name)?;
    draw_control_logic(&mut lib, ControlMode::Simple)?;

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
