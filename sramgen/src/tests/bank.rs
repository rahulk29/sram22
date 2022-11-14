use crate::layout::bank::*;
use crate::schematic::sram::*;
use crate::tests::{panic_on_err, test_gds_path, test_lef_path};
use crate::utils::save_modules;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sram_bank_16x16m2() -> Result<()> {
    let modules = sram(SramParams {
        name: "sramgen_sram_16x16m2".to_string(),
        row_bits: 4,
        col_bits: 4,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules("sram_16x16m2", modules)?;

    let mut lib = sky130::pdk_lib("test_sram_bank_16x16m2")?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 16,
            cols: 16,
            mux_ratio: 2,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;

    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    Ok(())
}

#[test]
fn test_sram_bank_32x32m2() -> Result<()> {
    let modules = sram(SramParams {
        name: "sramgen_sram_32x32m2".to_string(),
        row_bits: 5,
        col_bits: 5,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules("sram_32x32m2", modules)?;

    let mut lib = sky130::pdk_lib("test_sram_bank_32x32m2")?;
    let PhysicalDesign { cell: _, lef } = draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 32,
            cols: 32,
            mux_ratio: 2,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;
    lef.save(test_lef_path(&lib)).expect("failed to export LEF");

    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    Ok(())
}

#[test]
fn test_sram_bank_32x32m4() -> Result<()> {
    let modules = sram(SramParams {
        name: "sramgen_sram_32x32m4".to_string(),
        row_bits: 5,
        col_bits: 5,
        col_mask_bits: 2,
        wmask_groups: 1,
    });

    save_modules("sram_32x32m4", modules)?;

    let mut lib = sky130::pdk_lib("test_sram_bank_32x32m4")?;
    let PhysicalDesign { cell: _, lef } = draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 32,
            cols: 32,
            mux_ratio: 4,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;
    lef.save(test_lef_path(&lib)).expect("failed to export LEF");

    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    Ok(())
}

#[test]
fn test_sram_bank_32x32m8() -> Result<()> {
    let modules = sram(SramParams {
        name: "sramgen_sram_32x32m8".to_string(),
        row_bits: 5,
        col_bits: 5,
        col_mask_bits: 3,
        wmask_groups: 1,
    });

    save_modules("sram_32x32m8", modules)?;

    let mut lib = sky130::pdk_lib("test_sram_bank_32x32m8")?;
    let PhysicalDesign { cell: _, lef } = draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 32,
            cols: 32,
            mux_ratio: 8,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;
    lef.save(test_lef_path(&lib)).expect("failed to export LEF");

    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    Ok(())
}

#[test]
fn test_sram_bank_128x64m2() -> Result<()> {
    let modules = sram(SramParams {
        name: "sramgen_sram_128x64m2".to_string(),
        row_bits: 7,
        col_bits: 6,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules("sram_128x64m2", modules)?;

    let mut lib = sky130::pdk_lib("test_sram_bank_128x64m2")?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 128,
            cols: 64,
            mux_ratio: 2,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;

    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    Ok(())
}

#[test]
fn test_sram_bank_64x128m2() -> Result<()> {
    let modules = sram(SramParams {
        name: "sramgen_sram_128x64m2".to_string(),
        row_bits: 6,
        col_bits: 7,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules("sram_64x128m2", modules)?;

    let mut lib = sky130::pdk_lib("test_sram_bank_64x128m2")?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 64,
            cols: 128,
            mux_ratio: 2,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;

    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    Ok(())
}
