use crate::config::{ControlMode, SramConfig};

use crate::Result;

use super::generate_test;

#[cfg(feature = "calibre")]
pub(crate) mod calibre {
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
        let work_dir = PathBuf::from(BUILD_PATH).join(format!("drc/{}", name));

        let layout_path = test_gds_path(name);

        let data = run_drc(&DrcParams {
            cell_name: name.to_string(),
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

        let source_path_main = PathBuf::from(BUILD_PATH).join(format!("spice/{}.spice", name));
        let source_path_dff = PathBuf::from(LIB_PATH).join("openram_dff/openram_dff.spice");
        let source_path_sp_cell =
            PathBuf::from(LIB_PATH).join("sram_sp_cell/sky130_fd_bd_sram__sram_sp_cell.lvs.spice");
        let source_path_sp_sense_amp =
            PathBuf::from(LIB_PATH).join("sramgen_sp_sense_amp/sramgen_sp_sense_amp.spice");
        let source_path_control_simple =
            PathBuf::from(LIB_PATH).join("sramgen_control/sramgen_control_simple.spice");
        let work_dir = PathBuf::from(BUILD_PATH).join(format!("lvs/{}", name));

        assert!(
            matches!(
                run_lvs(&LvsParams {
                    work_dir,
                    layout_path,
                    layout_cell_name: name.to_string(),
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
    generate_test(SramConfig {
        num_words: 32,
        data_width: 8,
        mux_ratio: 2,
        write_size: 8,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_16x64m2w16_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 64,
        data_width: 16,
        mux_ratio: 2,
        write_size: 16,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_8x128m4w8_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 128,
        data_width: 8,
        mux_ratio: 4,
        write_size: 8,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_8x128m4w2_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 128,
        data_width: 8,
        mux_ratio: 4,
        write_size: 2,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_4x256m8w4_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 256,
        data_width: 4,
        mux_ratio: 8,
        write_size: 4,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_32x256m2w32_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 256,
        data_width: 32,
        mux_ratio: 2,
        write_size: 32,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_64x128m2w64_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 128,
        data_width: 64,
        mux_ratio: 2,
        write_size: 64,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_64x128m2w32_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 128,
        data_width: 64,
        mux_ratio: 2,
        write_size: 32,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_64x128m2w2_simple() -> Result<()> {
    generate_test(SramConfig {
        num_words: 128,
        data_width: 64,
        mux_ratio: 2,
        write_size: 2,
        control: ControlMode::Simple,
    })
}
