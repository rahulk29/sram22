use crate::layout::mux::read::*;
use crate::layout::mux::write::*;
use crate::tech::BITCELL_WIDTH;
use crate::tests::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_column_read_mux() -> Result<()> {
    let name = "sramgen_column_read_mux";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux(&mut lib)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_read_mux_2_array() -> Result<()> {
    let name = "sramgen_column_read_mux_2_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux_array(&mut lib, 64, 2)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_read_mux_4_array() -> Result<()> {
    let name = "sramgen_column_read_mux_4_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux_array(&mut lib, 64, 4)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_read_mux_8_array() -> Result<()> {
    let name = "sramgen_column_read_mux_8_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux_array(&mut lib, 64, 8)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_write_mux() -> Result<()> {
    let name = "sramgen_column_write_mux";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux(
        &mut lib,
        WriteMuxParams {
            width: BITCELL_WIDTH,
            wmask: false,
        },
    )?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_write_mux_wmask() -> Result<()> {
    let name = "sramgen_column_write_mux_wmask";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux(
        &mut lib,
        WriteMuxParams {
            width: BITCELL_WIDTH,
            wmask: true,
        },
    )?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_write_mux_array_m2() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m2";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(&mut lib, 32, 2, 1)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_write_mux_array_m4() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m4";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(&mut lib, 32, 4, 1)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_write_mux_array_m8() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m8";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(&mut lib, 32, 8, 1)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_column_write_mux_array_m4w4() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m4w4";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(&mut lib, 128, 4, 4)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}