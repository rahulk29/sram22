use crate::layout::tmc::*;
use crate::paths::out_gds;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_dbdr_delay_cell() -> Result<()> {
    let name = "sramgen_dbdr_delay_cell";
    let mut lib = sky130::pdk_lib(name)?;
    draw_dbdr_delay_cell(&mut lib, name)?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_sky130_tmc_unit_6() -> Result<()> {
    let name = "sramgen_tmc_unit_6";
    let mut lib = sky130::pdk_lib(name)?;
    draw_tmc_unit(
        &mut lib,
        TmcUnitParams {
            name: name.to_string(),
            multiplier: 6,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_sky130_tmc() -> Result<()> {
    let name = "sramgen_tmc";
    let mut lib = sky130::pdk_lib(name)?;
    draw_tmc(
        &mut lib,
        TmcParams {
            name: name.to_string(),
            multiplier: 6,
            units: 16,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}
