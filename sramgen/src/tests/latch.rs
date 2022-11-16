use crate::layout::latch::*;
use crate::tests::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_sr_latch() -> Result<()> {
    let name = "sramgen_sr_latch";
    let mut lib = sky130::pdk_lib(name)?;
    draw_sr_latch(&mut lib, name)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
