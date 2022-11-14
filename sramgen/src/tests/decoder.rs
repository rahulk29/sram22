use crate::layout::decoder::*;
use crate::schematic::decoder::DecoderTree;
use crate::schematic::gate::{GateParams, Size};
use crate::tech::BITCELL_HEIGHT;
use crate::utils::test_gds_path;
use crate::Result;
use layout21::raw::geom::Dir;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_inv_dec_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_inv_dec_array")?;
    draw_inv_dec_array(
        &mut lib,
        GateArrayParams {
            prefix: "inv_dec_array",
            width: 32,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_nand2_dec_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_nand2_dec_array")?;
    draw_inv_dec_array(
        &mut lib,
        GateArrayParams {
            prefix: "nand2_dec_array",
            width: 32,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_nand3_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_nand3_array")?;
    draw_nand3_array(
        &mut lib,
        GateArrayParams {
            prefix: "nand3_dec_array",
            width: 16,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
        GateParams {
            name: "nand3_dec_gate".to_string(),
            size: Size {
                nmos_width: 2_400,
                pmos_width: 2_000,
            },
            length: 150,
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_and3_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_and3_array")?;
    draw_and3_array(
        &mut lib,
        "test_sky130_and3_array",
        16,
        GateParams {
            name: "and3_nand".to_string(),
            size: Size {
                nmos_width: 2_400,
                pmos_width: 2_000,
            },
            length: 150,
        },
        GateParams {
            name: "and3_inv".to_string(),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 4_000,
            },
            length: 150,
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_and2_array() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_and2_array")?;
    draw_and2_array(
        &mut lib,
        "test_sky130_and2_array",
        16,
        GateParams {
            name: "and2_nand".to_string(),
            size: Size {
                nmos_width: 2_400,
                pmos_width: 2_000,
            },
            length: 150,
        },
        GateParams {
            name: "and2_inv".to_string(),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 4_000,
            },
            length: 150,
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_hier_decode_4bit() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_hier_decode_4bit")?;
    let tree = DecoderTree::new(4);
    draw_hier_decode(&mut lib, "hier_decode_4b", &tree.root)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_hier_decode_5bit() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_hier_decode_5bit")?;
    let tree = DecoderTree::new(5);
    draw_hier_decode(&mut lib, "hier_decode_5b", &tree.root)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_hier_decode_7bit() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_hier_decode_7bit")?;
    let tree = DecoderTree::new(7);
    draw_hier_decode(&mut lib, "hier_decode_7b", &tree.root)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
