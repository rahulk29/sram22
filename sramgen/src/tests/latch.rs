use crate::layout::latch::*;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_sr_latch() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_sr_latch")?;
    draw_sr_latch(&mut lib, "test_sky130_sr_latch")?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
