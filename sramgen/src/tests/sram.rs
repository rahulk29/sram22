use crate::layout::bank::*;
use crate::schematic::sram::*;
use crate::tests::{panic_on_err, test_gds_path, test_verilog_path};
use crate::utils::save_modules;
use crate::verilog::*;
use crate::{generate_netlist, Result};
use pdkprims::tech::sky130;

#[cfg(feature = "calibre")]
mod calibre {
    use crate::tests::test_gds_path;
    use crate::{Result, BUILD_PATH, LIB_PATH};
    use calibre::drc::{run_drc, DrcParams};
    use calibre::lvs::{run_lvs, LvsParams, LvsStatus};
    use calibre::RuleCheck;
    use std::path::PathBuf;

    const SKY130_DRC_RULES_PATH: &str = "/tools/B/rahulkumar/sky130/priv/drc/sram_drc_rules";
    const SKY130_LVS_RULES_PATH: &str =
        "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/LVS/Calibre/lvs_s8_opts";

    fn test_check_filter(check: &RuleCheck) -> bool {
        check.name.starts_with("r_") && check.name != "r_1252_metblk.6"
    }

    pub fn run_sram_drc_lvs(name: &str) -> Result<()> {
        let mut work_dir = PathBuf::from(BUILD_PATH);
        work_dir.push(format!("drc/{}", name));

        let layout_path = test_gds_path(name);

        let data = run_drc(&DrcParams {
            cell_name: "sram_bank".to_string(),
            work_dir,
            layout_path: layout_path.clone(),
            drc_rules_path: PathBuf::from(SKY130_DRC_RULES_PATH),
        })?;

        assert_eq!(
            data.rule_checks
                .into_iter()
                .filter(test_check_filter)
                .count(),
            0,
            "Found DRC errors"
        );

        let mut source_path_main = PathBuf::from(BUILD_PATH);
        source_path_main.push(format!("spice/{}.spice", name));
        let mut source_path_dff = PathBuf::from(LIB_PATH);
        source_path_dff.push("openram_dff/openram_dff.spice");
        let mut source_path_sp_cell = PathBuf::from(LIB_PATH);
        source_path_sp_cell.push("sram_sp_cell/sky130_fd_bd_sram__sram_sp_cell.lvs.spice");
        let mut source_path_sp_sense_amp = PathBuf::from(LIB_PATH);
        source_path_sp_sense_amp.push("sramgen_sp_sense_amp/sramgen_sp_sense_amp.spice");
        let mut source_path_control_simple = PathBuf::from(LIB_PATH);
        source_path_control_simple.push("sramgen_control/sramgen_control_simple.spice");
        let mut work_dir = PathBuf::from(BUILD_PATH);
        work_dir.push(format!("lvs/{}", name));

        assert!(
            matches!(
                run_lvs(&LvsParams {
                    work_dir,
                    layout_path,
                    layout_cell_name: "sram_bank".to_string(),
                    source_paths: vec![
                        source_path_main,
                        source_path_dff,
                        source_path_sp_cell,
                        source_path_sp_sense_amp,
                        source_path_control_simple,
                    ],
                    source_cell_name: name.to_string(),
                    lvs_rules_path: PathBuf::from(SKY130_LVS_RULES_PATH),
                })?
                .status,
                LvsStatus::Correct,
            ),
            "LVS failed"
        );

        Ok(())
    }
}

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

    lib.save_gds(test_gds_path(name)).map_err(panic_on_err)?;

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

    #[cfg(feature = "calibre")]
    self::calibre::run_sram_drc_lvs(name)?;

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
    lib.save_gds(test_gds_path(name)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 64,
            data_width: 16,
            addr_width: 6,
        },
    )?;

    #[cfg(feature = "calibre")]
    self::calibre::run_sram_drc_lvs(name)?;

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
    lib.save_gds(test_gds_path(name)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 128,
            data_width: 8,
            addr_width: 7,
        },
    )?;

    #[cfg(feature = "calibre")]
    self::calibre::run_sram_drc_lvs(name)?;

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

    lib.save_gds(test_gds_path(name)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 256,
            data_width: 4,
            addr_width: 8,
        },
    )?;

    #[cfg(feature = "calibre")]
    self::calibre::run_sram_drc_lvs(name)?;

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

    lib.save_gds(test_gds_path(name)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 256,
            data_width: 32,
            addr_width: 8,
        },
    )?;

    #[cfg(feature = "calibre")]
    self::calibre::run_sram_drc_lvs(name)?;

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

    lib.save_gds(test_gds_path(name)).map_err(panic_on_err)?;

    save_1rw_verilog(
        test_verilog_path(name),
        Sram1RwParams {
            module_name: name.to_string(),
            num_words: 128,
            data_width: 64,
            addr_width: 7,
        },
    )?;

    #[cfg(feature = "calibre")]
    self::calibre::run_sram_drc_lvs(name)?;

    Ok(())
}
