use crate::layout::gate::*;
use crate::schematic::gate::{GateParams, Size};
use crate::utils::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sky130_nand2_dec() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_nand2_dec")?;
    draw_nand2_dec(&mut lib, "nand2_dec")?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_inv_dec() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_inv_dec")?;
    draw_inv_dec(&mut lib, "inv_dec")?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_and2() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_and2")?;
    draw_and2(
        &mut lib,
        AndParams {
            name: "sky130_and2".to_string(),
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
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_nor2() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_nor2")?;
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

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_nand3() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_nand3")?;
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

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_sky130_and3() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_sky130_and3")?;
    draw_and3(
        &mut lib,
        AndParams {
            name: "sky130_and3".to_string(),
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

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
