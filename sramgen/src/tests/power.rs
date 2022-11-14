use crate::layout::col_inv::*;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_col_inv_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_col_inv_array")?;
    draw_col_inv_array(&mut lib, "test_col_inv_array", 32, 2)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
