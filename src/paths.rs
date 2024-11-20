use std::path::{Path, PathBuf};

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
