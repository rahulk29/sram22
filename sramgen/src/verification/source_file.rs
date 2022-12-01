use std::path::{Path, PathBuf};

use crate::config::sram::ControlMode;
use crate::verification::VerificationTask;
use crate::LIB_PATH;

pub fn sram_source_files(
    work_dir: impl AsRef<Path>,
    name: &str,
    task: VerificationTask,
    control_mode: ControlMode,
) -> Vec<PathBuf> {
    let source_path_main = match task {
        VerificationTask::SpectreSim => {
            PathBuf::from(work_dir.as_ref()).join(format!("{}.spectre.spice", name))
        }
        VerificationTask::NgspiceSim => {
            PathBuf::from(work_dir.as_ref()).join(format!("{}.ngspice.spice", name))
        }
        _ => PathBuf::from(work_dir.as_ref()).join(format!("{}.spice", name)),
    };
    let source_path_dff = PathBuf::from(LIB_PATH).join("openram_dff/openram_dff.spice");
    let source_path_sp_cell = match task {
        VerificationTask::SpiceSim
        | VerificationTask::NgspiceSim
        | VerificationTask::SpectreSim => {
            PathBuf::from(LIB_PATH).join("sram_sp_cell/sky130_fd_bd_sram__sram_sp_cell.spice")
        }
        VerificationTask::Lvs | VerificationTask::Pex => {
            PathBuf::from(LIB_PATH).join("sram_sp_cell/sky130_fd_bd_sram__sram_sp_cell.lvs.spice")
        }
    };
    let source_path_sp_replica_cell = match task {
        VerificationTask::SpiceSim
        | VerificationTask::NgspiceSim
        | VerificationTask::SpectreSim => PathBuf::from(LIB_PATH)
            .join("sram_sp_cell_replica/sky130_fd_bd_sram__openram_sp_cell_opt1_replica.spice"),
        VerificationTask::Lvs | VerificationTask::Pex => PathBuf::from(LIB_PATH)
            .join("sram_sp_cell_replica/sky130_fd_bd_sram__openram_sp_cell_opt1_replica.lvs.spice"),
    };
    let source_path_sp_sense_amp =
        PathBuf::from(LIB_PATH).join("sramgen_sp_sense_amp/sramgen_sp_sense_amp.spice");

    let source_path_control = match control_mode {
        ControlMode::Simple => {
            PathBuf::from(LIB_PATH).join("sramgen_control/sramgen_control_simple.spice")
        }
        ControlMode::ReplicaV1 => {
            PathBuf::from(LIB_PATH).join("sramgen_control/sramgen_control_replica_v1.spice")
        }
    };

    vec![
        source_path_main,
        source_path_dff,
        source_path_sp_cell,
        source_path_sp_replica_cell,
        source_path_sp_sense_amp,
        source_path_control,
    ]
}

pub fn bitcell_array_source_files(
    work_dir: impl AsRef<Path>,
    name: &str,
    task: VerificationTask,
) -> Vec<PathBuf> {
    let source_path_main = match task {
        VerificationTask::SpectreSim => {
            PathBuf::from(work_dir.as_ref()).join(format!("{}.spectre.spice", name))
        }
        VerificationTask::NgspiceSim => {
            PathBuf::from(work_dir.as_ref()).join(format!("{}.ngspice.spice", name))
        }
        _ => PathBuf::from(work_dir.as_ref()).join(format!("{}.spice", name)),
    };
    let source_path_sp_cell = match task {
        VerificationTask::SpiceSim
        | VerificationTask::NgspiceSim
        | VerificationTask::SpectreSim => {
            PathBuf::from(LIB_PATH).join("sram_sp_cell/sky130_fd_bd_sram__sram_sp_cell.spice")
        }
        VerificationTask::Lvs | VerificationTask::Pex => {
            PathBuf::from(LIB_PATH).join("sram_sp_cell/sky130_fd_bd_sram__sram_sp_cell.lvs.spice")
        }
    };
    let source_path_sp_replica_cell = match task {
        VerificationTask::SpiceSim
        | VerificationTask::NgspiceSim
        | VerificationTask::SpectreSim => PathBuf::from(LIB_PATH)
            .join("sram_sp_cell_replica/sky130_fd_bd_sram__openram_sp_cell_opt1_replica.spice"),
        VerificationTask::Lvs | VerificationTask::Pex => PathBuf::from(LIB_PATH)
            .join("sram_sp_cell_replica/sky130_fd_bd_sram__openram_sp_cell_opt1_replica.lvs.spice"),
    };

    vec![
        source_path_main,
        source_path_sp_cell,
        source_path_sp_replica_cell,
    ]
}
