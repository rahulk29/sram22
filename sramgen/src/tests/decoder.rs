use crate::layout::decoder::*;
use crate::schematic::decoder::*;
use crate::schematic::gate::{GateParams, Size};
use crate::tech::BITCELL_HEIGHT;
use crate::tests::test_gds_path;
use crate::utils::save_modules;
use crate::{generate_netlist, Result};
use layout21::raw::geom::Dir;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_inv_dec_array() -> Result<()> {
    let name = "sramgen_inv_dec_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_dec_array(
        &mut lib,
        GateArrayParams {
            prefix: "inv_dec_array",
            width: 32,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_nand2_dec_array() -> Result<()> {
    let name = "sramgen_nand2_dec_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_dec_array(
        &mut lib,
        GateArrayParams {
            prefix: "nand2_dec_array",
            width: 32,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_nand3_array() -> Result<()> {
    let name = "sramgen_nand3_array";
    let mut lib = sky130::pdk_lib(name)?;
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

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_and3_array() -> Result<()> {
    let name = "sramgen_and3_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_and3_array(
        &mut lib,
        name,
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

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_and2_array() -> Result<()> {
    let name = "sramgen_and2_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_and2_array(
        &mut lib,
        name,
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

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_hier_decode_4bit() -> Result<()> {
    let name = "sramgen_hier_decoder_4bit";
    let tree = DecoderTree::new(4);

    let decoder_params = DecoderParams {
        tree: tree.clone(),
        lch: 150,
        name: name.to_string(),
    };
    let modules = hierarchical_decoder(decoder_params);

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_hier_decode(&mut lib, name, &tree.root)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_hier_decode_5bit() -> Result<()> {
    let name = "sramgen_hier_decoder_5bit";
    let tree = DecoderTree::new(5);

    let decoder_params = DecoderParams {
        tree: tree.clone(),
        lch: 150,
        name: name.to_string(),
    };
    let modules = hierarchical_decoder(decoder_params);

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_hier_decode(&mut lib, name, &tree.root)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_hier_decode_7bit() -> Result<()> {
    let name = "sramgen_hier_decoder_7bit";
    let tree = DecoderTree::new(7);

    let decoder_params = DecoderParams {
        tree: tree.clone(),
        lch: 150,
        name: name.to_string(),
    };
    let modules = hierarchical_decoder(decoder_params);

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_hier_decode(&mut lib, name, &tree.root)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_get_idxs() {
    let bases = [4, 8, 5];
    let idxs = get_idxs(14, &bases);
    assert_eq!(idxs, [0, 2, 4]);
    let idxs = get_idxs(40, &bases);
    assert_eq!(idxs, [1, 0, 0]);
}
