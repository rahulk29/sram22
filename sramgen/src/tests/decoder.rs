use crate::config::decoder::*;
use crate::config::gate::{GateParams, Size};
use crate::layout::decoder::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::decoder::*;
use crate::schematic::{generate_netlist, save_modules};
use crate::tech::BITCELL_HEIGHT;
use crate::tests::test_work_dir;
use crate::Result;
use layout21::raw::geom::Dir;
use pdkprims::tech::sky130;

#[test]
fn test_inv_dec_array() -> Result<()> {
    let name = "sramgen_inv_dec_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_dec_array(
        &mut lib,
        &GateDecArrayParams {
            name: "inv_dec_array".to_string(),
            width: 32,
            dir: Dir::Vert,
            pitch: Some(BITCELL_HEIGHT),
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_nand2_dec_array() -> Result<()> {
    let name = "sramgen_nand2_dec_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_nand_dec_array(
        &mut lib,
        &NandDecArrayParams {
            array_params: GateDecArrayParams {
                name: "nand2_dec_array".to_string(),
                width: 32,
                dir: Dir::Vert,
                pitch: Some(BITCELL_HEIGHT),
            },
            gate: GateParams {
                name: "nand3_dec_gate".to_string(),
                size: Size {
                    nmos_width: 2_400,
                    pmos_width: 2_000,
                },
                length: 150,
            },
            gate_size: 2,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_nand3_array() -> Result<()> {
    let name = "sramgen_nand3_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_nand_dec_array(
        &mut lib,
        &NandDecArrayParams {
            array_params: GateDecArrayParams {
                name: "nand3_dec_array".to_string(),
                width: 16,
                dir: Dir::Vert,
                pitch: Some(BITCELL_HEIGHT),
            },
            gate: GateParams {
                name: "nand3_dec_gate".to_string(),
                size: Size {
                    nmos_width: 2_400,
                    pmos_width: 2_000,
                },
                length: 150,
            },
            gate_size: 3,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_and3_array() -> Result<()> {
    let name = "sramgen_and3_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_and_dec_array(
        &mut lib,
        &AndDecArrayParams {
            array_params: GateDecArrayParams {
                name: name.to_string(),
                width: 16,
                dir: Dir::Vert,
                pitch: None,
            },
            nand: GateParams {
                name: "and3_nand".to_string(),
                size: Size {
                    nmos_width: 2_400,
                    pmos_width: 2_000,
                },
                length: 150,
            },
            inv: GateParams {
                name: "and3_inv".to_string(),
                size: Size {
                    nmos_width: 2_000,
                    pmos_width: 4_000,
                },
                length: 150,
            },
            gate_size: 3,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_and2_array() -> Result<()> {
    let name = "sramgen_and2_array";
    let mut lib = sky130::pdk_lib(name)?;
    draw_and_dec_array(
        &mut lib,
        &AndDecArrayParams {
            array_params: GateDecArrayParams {
                name: name.to_string(),
                width: 16,
                dir: Dir::Vert,
                pitch: None,
            },
            nand: GateParams {
                name: "and3_nand".to_string(),
                size: Size {
                    nmos_width: 2_400,
                    pmos_width: 2_000,
                },
                length: 150,
            },
            inv: GateParams {
                name: "and3_inv".to_string(),
                size: Size {
                    nmos_width: 2_000,
                    pmos_width: 4_000,
                },
                length: 150,
            },
            gate_size: 2,
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_hier_decode_4bit() -> Result<()> {
    let name = "sramgen_hier_decoder_4bit";
    let tree = DecoderTree::new(4);

    let decoder_params = DecoderParams {
        tree: tree.clone(),
        lch: 150,
        name: name.to_string(),
    };
    let modules = hierarchical_decoder(decoder_params);

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_hier_decode(&mut lib, name, &tree.root)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_hier_decode_5bit() -> Result<()> {
    let name = "sramgen_hier_decoder_5bit";
    let tree = DecoderTree::new(5);

    let decoder_params = DecoderParams {
        tree: tree.clone(),
        lch: 150,
        name: name.to_string(),
    };
    let modules = hierarchical_decoder(decoder_params);

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_hier_decode(&mut lib, name, &tree.root)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_hier_decode_7bit() -> Result<()> {
    let name = "sramgen_hier_decoder_7bit";
    let tree = DecoderTree::new(7);

    let decoder_params = DecoderParams {
        tree: tree.clone(),
        lch: 150,
        name: name.to_string(),
    };
    let modules = hierarchical_decoder(decoder_params);

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_hier_decode(&mut lib, name, &tree.root)?;

    lib.save_gds(out_gds(&work_dir, name))?;

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
