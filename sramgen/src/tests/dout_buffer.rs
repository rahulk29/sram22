use crate::layout::dout_buffer::*;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_dout_buffer_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_dout_buffer_array")?;
    draw_dout_buffer_array(&mut lib, "test_dout_buffer_array", 32, 2)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_dout_buffer() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_dout_buffer")?;
    draw_dout_buffer(&mut lib, "test_dout_buffer")?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
