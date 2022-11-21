use std::path::{Path, PathBuf};

pub use anyhow::{anyhow, Result};

pub mod cli;
pub mod config;
pub mod layout;
pub mod plan;
pub mod schematic;
pub mod tech;
#[cfg(test)]
mod tests;
pub mod verification;
pub mod verilog;

pub const BUILD_PATH: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build");
pub const LIB_PATH: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib");

pub fn out_bin(work_dir: impl AsRef<Path>, name: &str) -> PathBuf {
    PathBuf::from(work_dir.as_ref()).join("pb").join(name)
}

#[inline]
pub(crate) fn clog2(x: usize) -> usize {
    (x as f64).log2().ceil() as usize
}
