use crate::config::mux::{ReadMuxArrayParams, ReadMuxParams, WriteMuxArrayParams, WriteMuxParams};
use crate::layout::mux::read::*;
use crate::layout::mux::write::*;
use crate::paths::out_gds;
use crate::tech::BITCELL_WIDTH;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_column_read_mux() -> Result<()> {
    let name = "sramgen_column_read_mux";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux(
        &mut lib,
        &ReadMuxParams {
            name: name.to_string(),
            length: 150,
            width: 1_200,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_read_mux_2_array() -> Result<()> {
    let name = "sramgen_column_read_mux_2_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux_array(
        &mut lib,
        &ReadMuxArrayParams {
            name: name.to_string(),
            mux_params: ReadMuxParams {
                name: "read_mux".to_string(),
                length: 150,
                width: 1_200,
            },
            cols: 64,
            mux_ratio: 2,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_read_mux_4_array() -> Result<()> {
    let name = "sramgen_column_read_mux_4_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux_array(
        &mut lib,
        &ReadMuxArrayParams {
            name: name.to_string(),
            mux_params: ReadMuxParams {
                name: "read_mux".to_string(),
                length: 150,
                width: 1_200,
            },
            cols: 64,
            mux_ratio: 4,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_read_mux_8_array() -> Result<()> {
    let name = "sramgen_column_read_mux_8_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_read_mux_array(
        &mut lib,
        &ReadMuxArrayParams {
            name: name.to_string(),
            mux_params: ReadMuxParams {
                name: "read_mux".to_string(),
                length: 150,
                width: 1_200,
            },
            cols: 64,
            mux_ratio: 8,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_write_mux() -> Result<()> {
    let name = "sramgen_column_write_mux";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux(
        &mut lib,
        &WriteMuxParams {
            name: name.to_string(),
            length: 150,
            width: BITCELL_WIDTH,
            wmask: false,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_write_mux_wmask() -> Result<()> {
    let name = "sramgen_column_write_mux_wmask";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux(
        &mut lib,
        &WriteMuxParams {
            name: name.to_string(),
            length: 150,
            width: BITCELL_WIDTH,
            wmask: true,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_write_mux_array_m2() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m2";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(
        &mut lib,
        &WriteMuxArrayParams {
            name: name.to_string(),
            mux_params: WriteMuxParams {
                name: "write_mux".to_string(),
                length: 150,
                width: BITCELL_WIDTH,
                wmask: true,
            },
            cols: 32,
            mux_ratio: 2,
            wmask_width: 1,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_write_mux_array_m4() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m4";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(
        &mut lib,
        &WriteMuxArrayParams {
            name: name.to_string(),
            mux_params: WriteMuxParams {
                name: "write_mux".to_string(),
                length: 150,
                width: BITCELL_WIDTH,
                wmask: true,
            },
            cols: 32,
            mux_ratio: 4,
            wmask_width: 1,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_write_mux_array_m8() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m8";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(
        &mut lib,
        &WriteMuxArrayParams {
            name: name.to_string(),
            mux_params: WriteMuxParams {
                name: "write_mux".to_string(),
                length: 150,
                width: BITCELL_WIDTH,
                wmask: true,
            },
            cols: 32,
            mux_ratio: 8,
            wmask_width: 1,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_column_write_mux_array_m4w4() -> Result<()> {
    let name = "sramgen_column_write_mux_array_m4w4";
    let mut lib = sky130::pdk_lib(name)?;
    draw_write_mux_array(
        &mut lib,
        &WriteMuxArrayParams {
            name: name.to_string(),
            mux_params: WriteMuxParams {
                name: "write_mux".to_string(),
                length: 150,
                width: BITCELL_WIDTH,
                wmask: true,
            },
            cols: 128,
            mux_ratio: 4,
            wmask_width: 4,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}
