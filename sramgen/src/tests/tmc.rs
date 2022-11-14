use crate::layout::tmc::*;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_dbdr_delay_cell() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_dbdr_delay_cell")?;
    draw_dbdr_delay_cell(&mut lib, "test_sky130_dbdr_delay_cell")?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_tmc_unit_6() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_tmc_unit_6")?;
    draw_tmc_unit(
        &mut lib,
        TmcUnitParams {
            name: "test_sky130_tmc_unit_6".to_string(),
            multiplier: 6,
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_tmc() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_tmc")?;
    draw_tmc(
        &mut lib,
        TmcParams {
            name: "test_sky130_tmc".to_string(),
            multiplier: 6,
            units: 16,
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
