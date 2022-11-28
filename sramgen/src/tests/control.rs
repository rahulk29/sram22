use crate::config::ControlMode;
use crate::layout::control::*;
use crate::paths::out_gds;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_control_logic_simple() -> Result<()> {
    let name = "sramgen_control_logic_simple";
    let mut lib = sky130::pdk_lib(name)?;
    draw_control_logic(&mut lib, ControlMode::Simple)?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_control_logic_replica_v1() -> Result<()> {
    let name = "sramgen_control_logic_replica_v1";
    let mut lib = sky130::pdk_lib(name)?;
    draw_control_logic_replica_v1(&mut lib)?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}
