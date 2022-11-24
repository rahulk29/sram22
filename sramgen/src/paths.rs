use std::path::{Path, PathBuf};

pub fn out_bin(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{}.pb.bin", name))
}

pub fn out_gds(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{}.gds", name))
}

pub fn out_verilog(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{}.gds", name))
}

#[cfg(feature = "abstract_lef")]
pub fn out_lef(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join(format!("{}.lef", name))
}
