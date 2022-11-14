use crate::layout::dff::*;
use crate::tech::COLUMN_WIDTH;
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_dff_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_dff_array")?;
    draw_dff_array(&mut lib, "test_sky130_dff_array", 16)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_vert_dff_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_vert_dff_array")?;
    draw_vert_dff_array(&mut lib, "test_sky130_vert_dff_array", 8)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_dff_grid() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_dff_grid")?;
    let params = DffGridParams::builder()
        .name("test_sky130_dff_grid")
        .rows(4)
        .cols(8)
        .row_pitch(4 * COLUMN_WIDTH)
        .build()?;
    draw_dff_grid(&mut lib, params)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
