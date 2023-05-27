use crate::config::sram::SramConfig;
use std::path::{Path, PathBuf};

pub fn out_sram(config: &SramConfig) -> String {
    let &SramConfig {
        num_words,
        data_width,
        mux_ratio,
        write_size,
        control,
        ..
    } = config;
    format!("sramgen_sram_{num_words}x{data_width}m{mux_ratio}w{write_size}_{control}")
}

pub fn out_bin(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{name}.pb.bin"))
}

pub fn out_spice(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{name}.spice"))
}

pub fn out_gds(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{name}.gds"))
}

pub fn out_verilog(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{name}.v"))
}

#[cfg(feature = "commercial")]
pub fn out_lef(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{name}.lef"))
}

#[cfg(feature = "commercial")]
pub fn out_lib(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{name}.lib"))
}

#[cfg(feature = "commercial")]
use calibre::pex::PexLevel;

#[cfg(feature = "commercial")]
pub fn out_pex(work_dir: impl AsRef<Path>, name: &str, level: PexLevel) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{name}.{level}.pex.netlist"))
}
