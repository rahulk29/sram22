use crate::layout::gate::*;
use crate::schematic::gate::*;
use crate::schematic::gate::{GateParams, Size};
use crate::tests::test_gds_path;
use crate::utils::save_modules;
use crate::{generate_netlist, Result};
use pdkprims::tech::sky130;

#[test]
fn test_sky130_nand2_dec() -> Result<()> {
    let name = "sramgen_nand2_dec";
    let mut lib = sky130::pdk_lib(name)?;
    draw_nand2_dec(&mut lib, name)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_inv_dec() -> Result<()> {
    let name = "sramgen_inv_dec";
    let mut lib = sky130::pdk_lib(name)?;
    draw_inv_dec(&mut lib, name)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_and2() -> Result<()> {
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

    save_modules(name, and2)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_and2(&mut lib, params)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_nor2() -> Result<()> {
    let name = "sramgen_nor2";
    let mut lib = sky130::pdk_lib(name)?;
    draw_nor2(
        &mut lib,
        GateParams {
            name: "test_sky130_nor2".to_string(),
            size: Size {
                nmos_width: 1_200,
                pmos_width: 3_000,
            },
            length: 150,
        },
    )?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_nand3() -> Result<()> {
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

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_sky130_and3() -> Result<()> {
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

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
