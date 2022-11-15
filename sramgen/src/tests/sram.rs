use crate::layout::bank::*;
use crate::schematic::sram::*;
use crate::tests::{panic_on_err, test_gds_path, test_verilog_path};
use crate::utils::save_modules;
use crate::verilog::*;
use crate::{generate_netlist, Result};
use pdkprims::tech::sky130;

#[test]
fn test_sram_8x32m2w8_simple() -> Result<()> {
    let name = "sramgen_sram_8x32m2w8_simple";
    let modules = sram(SramParams {
        name: name.to_string(),
        row_bits: 4,
        col_bits: 4,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
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

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 32,
            data_width: 8,
            addr_width: 5,
        },
    )
    .unwrap();

    Ok(())
}

#[test]
fn test_sram_16x64m2w8_simple() -> Result<()> {
    let name = "sramgen_sram_16x64m2w8_simple";
    let modules = sram(SramParams {
        name: name.to_string(),
        row_bits: 5,
        col_bits: 5,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 32,
            cols: 32,
            mux_ratio: 2,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;
    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 64,
            data_width: 16,
            addr_width: 6,
        },
    )?;

    Ok(())
}

#[test]
fn test_sram_8x128m4w8_simple() -> Result<()> {
    let name = "sramgen_sram_8x128m4w8_simple";
    let modules = sram(SramParams {
        name: name.to_string(),
        row_bits: 5,
        col_bits: 5,
        col_mask_bits: 2,
        wmask_groups: 1,
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 32,
            cols: 32,
            mux_ratio: 4,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;
    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 128,
            data_width: 8,
            addr_width: 7,
        },
    )?;

    Ok(())
}

#[test]
fn test_sram_4x256m8w8_simple() -> Result<()> {
    let name = "sramgen_sram_4x256m8w8_simple";
    let modules = sram(SramParams {
        name: name.to_string(),
        row_bits: 5,
        col_bits: 5,
        col_mask_bits: 3,
        wmask_groups: 1,
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            rows: 32,
            cols: 32,
            mux_ratio: 8,
            wmask_groups: 1,
        },
    )
    .map_err(panic_on_err)?;

    lib.save_gds(test_gds_path(&lib)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 256,
            data_width: 4,
            addr_width: 8,
        },
    )?;

    Ok(())
}

#[test]
fn test_sram_32x256m2w8_simple() -> Result<()> {
    let name = "sramgen_sram_32x256m2w8_simple";
    let modules = sram(SramParams {
        name: name.to_string(),
        row_bits: 7,
        col_bits: 6,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
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

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 256,
            data_width: 32,
            addr_width: 8,
        },
    )?;

    Ok(())
}

#[test]
fn test_sram_64x128m2w8_simple() -> Result<()> {
    let name = "sramgen_sram_64x128m2w8_simple";
    let modules = sram(SramParams {
        name: name.to_string(),
        row_bits: 6,
        col_bits: 7,
        col_mask_bits: 1,
        wmask_groups: 1,
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
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

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 128,
            data_width: 64,
            addr_width: 7,
        },
    )?;

    Ok(())
}
