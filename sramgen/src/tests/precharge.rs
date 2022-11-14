use crate::layout::precharge::*;
use crate::schematic::precharge::{PrechargeArrayParams, PrechargeParams};
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_precharge() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_precharge")?;
    draw_precharge(
        &mut lib,
        PrechargeParams {
            name: "test_sky130_precharge".to_string(),
            length: 150,
            pull_up_width: 1_200,
            equalizer_width: 1_000,
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_precharge_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_precharge_array")?;
    draw_precharge_array(
        &mut lib,
        PrechargeArrayParams {
            width: 32,
            instance_params: PrechargeParams {
                name: "precharge".to_string(),
                length: 150,
                pull_up_width: 1_200,
                equalizer_width: 1_000,
            },
            name: "precharge_array".to_string(),
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
