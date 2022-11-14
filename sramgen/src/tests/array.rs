use crate::layout::array::*;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sram_array_32x32() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sram_array_32x32")?;
    draw_array(32, 32, &mut lib)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sram_array_2x2() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sram_array_2x2")?;
    draw_array(2, 2, &mut lib)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
