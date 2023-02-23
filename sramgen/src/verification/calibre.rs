use crate::paths::out_gds;
use crate::verification::{source_files, VerificationTask};
use crate::Result;
use anyhow::bail;
use calibre::drc::{run_drc, DrcParams};
use calibre::lvs::{run_lvs, LvsParams, LvsStatus};
#[cfg(feature = "pex")]
use calibre::pex::{run_pex, PexParams};
use calibre::RuleCheck;
use std::path::{Path, PathBuf};

pub(crate) const SKY130_DRC_RULES_PATH: &str =
    "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/DRC/Calibre/s8_drcRules";
pub(crate) const SKY130_DRC_RUNSET_PATH: &str = "/tools/B/rahulkumar/sky130/priv/drc/runset";
pub(crate) const SKY130_LVS_RULES_PATH: &str =
    "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/LVS/Calibre/lvs_s8_opts";
pub(crate) const SKY130_PEX_RULES_PATH: &str =
    "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/PEX/xRC/xrcControlFile_s8";

fn test_check_filter(check: &RuleCheck) -> bool {
    check.name.starts_with("r_") && check.name != "r_1252_metblk.6"
}

pub fn run_sram_drc(work_dir: impl AsRef<Path>, name: &str) -> Result<()> {
    let drc_work_dir = PathBuf::from(work_dir.as_ref()).join("drc");

    let layout_path = out_gds(&work_dir, name);

    let data = run_drc(&DrcParams {
        cell_name: name,
        work_dir: &drc_work_dir,
        layout_path: &layout_path,
        rules_path: &PathBuf::from(SKY130_DRC_RULES_PATH),
        runset_path: Some(&PathBuf::from(SKY130_DRC_RUNSET_PATH)),
    })?;

    if data
        .rule_checks
        .into_iter()
        .filter(test_check_filter)
        .count()
        > 0
    {
        bail!("Found DRC errors");
    }

    Ok(())
}

pub fn run_sram_lvs(
    work_dir: impl AsRef<Path>,
    name: &str,
    control_mode: crate::config::sram::ControlMode,
) -> Result<()> {
    let lvs_work_dir = PathBuf::from(work_dir.as_ref()).join("lvs");

    let layout_path = out_gds(&work_dir, name);

    if run_lvs(&LvsParams {
        work_dir: &lvs_work_dir,
        layout_path: &layout_path,
        layout_cell_name: name,
        source_paths: &source_files(&work_dir, name, VerificationTask::Lvs, control_mode),
        source_cell_name: name,
        rules_path: &PathBuf::from(SKY130_LVS_RULES_PATH),
    })?
    .status
        != LvsStatus::Correct
    {
        bail!("LVS failed");
    }

    Ok(())
}

#[cfg(feature = "pex")]
pub fn run_sram_pex(
    work_dir: impl AsRef<Path>,
    pex_netlist_path: impl AsRef<Path>,
    name: &str,
    control_mode: crate::config::sram::ControlMode,
) -> Result<()> {
    let pex_work_dir = PathBuf::from(work_dir.as_ref()).join("pex");
    let pex_netlist_path = pex_netlist_path.as_ref();

    let layout_path = out_gds(&work_dir, name);

    if run_pex(&PexParams {
        work_dir: &pex_work_dir,
        layout_path: &layout_path,
        layout_cell_name: name,
        source_paths: &source_files(&work_dir, name, VerificationTask::Pex, control_mode),
        source_cell_name: name,
        rules_path: &PathBuf::from(SKY130_PEX_RULES_PATH),
        pex_netlist_path,
    })?
    .status
        != LvsStatus::Correct
    {
        bail!("PEX LVS failed");
    }

    Ok(())
}
