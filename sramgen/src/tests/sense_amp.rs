use crate::layout::sense_amp::*;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_sense_amp_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_sense_amp_array")?;
    draw_sense_amp_array(&mut lib, 16, 2 * 2_500)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
