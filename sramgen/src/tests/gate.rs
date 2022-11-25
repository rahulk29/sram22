use crate::layout::gate::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::gate::{GateParams, Size, *};
use crate::schematic::{generate_netlist, save_modules};
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_nand2_dec() -> Result<()> {
    let name = "sramgen_nand2_dec";
    let mut lib = sky130::pdk_lib(name)?;
    draw_nand2_dec(&mut lib, name)?;

    let work_dir = test_work_dir(name);

    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_inv_dec() -> Result<()> {
    let name = "sramgen_inv_dec";
    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_dec(&mut lib, name)?;

    let work_dir = test_work_dir(name);

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_and2() -> Result<()> {
    let name = "sramgen_and2";
    let params = AndParams {
        name: name.to_string(),
        nand: GateParams {
            name: "and2_nand".to_string(),
            length: 150,
            size: Size {
                pmos_width: 2_400,
                nmos_width: 1_800,
            },
        },
        inv: GateParams {
            name: "and2_inv".to_string(),
            length: 150,
            size: Size {
                pmos_width: 2_400,
                nmos_width: 1_800,
            },
        },
    };

    let and2 = and2(params.clone());

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, and2)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_and2(&mut lib, params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_nor2() -> Result<()> {
    let name = "sramgen_nor2";
    let mut lib = sky130::pdk_lib(name)?;
    draw_nor2(
        &mut lib,
        GateParams {
            name: "sramgen_nor2".to_string(),
            size: Size {
                nmos_width: 1_200,
                pmos_width: 3_000,
            },
            length: 150,
        },
    )?;

    let work_dir = test_work_dir(name);

    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_nand3() -> Result<()> {
    let name = "sramgen_nand3";
    let mut lib = sky130::pdk_lib(name)?;
    draw_nand3(
        &mut lib,
        GateParams {
            name: "nand3".to_string(),
            size: Size {
                nmos_width: 1_600,
                pmos_width: 2_400,
            },
            length: 150,
        },
    )?;

    let work_dir = test_work_dir(name);

    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}

#[test]
fn test_and3() -> Result<()> {
    let name = "sramgen_and3";
    let mut lib = sky130::pdk_lib(name)?;
    draw_and3(
        &mut lib,
        AndParams {
            name: name.to_string(),
            nand: GateParams {
                name: "and3_nand".to_string(),
                length: 150,
                size: Size {
                    pmos_width: 2_400,
                    nmos_width: 2_800,
                },
            },
            inv: GateParams {
                name: "and3_inv".to_string(),
                length: 150,
                size: Size {
                    pmos_width: 2_400,
                    nmos_width: 1_800,
                },
            },
        },
    )?;

    let work_dir = test_work_dir(name);

    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}
