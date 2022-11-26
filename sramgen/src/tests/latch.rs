use crate::layout::latch::*;
use crate::paths::out_gds;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sr_latch() -> Result<()> {
    let name = "sramgen_sr_latch";
    let mut lib = sky130::pdk_lib(name)?;
    draw_sr_latch(&mut lib, name)?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(&work_dir, name))?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sr_latch(&mut lib, name)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
