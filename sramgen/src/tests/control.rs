use crate::config::ControlMode;
use crate::layout::control::*;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_control_logic_simple() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_control_logic_simple")?;
    draw_control_logic(&mut lib, ControlMode::Simple)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_inv_chain_12() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_inv_chain_12")?;
    draw_inv_chain(
        &mut lib,
        InvChainParams {
            prefix: "test_sky130_inv_chain_12",
            num: 12,
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
