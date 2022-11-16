use std::path::PathBuf;

use layout21::raw::{BoundBox, Cell};
use layout21::utils::Ptr;
use schematic::mos::NetlistFormat;
use vlsir::circuit::Package;
use vlsir::spice::SimInput;

use std::process::{Command, Stdio};

pub mod cli;
pub mod config;
pub mod layout;
pub mod plan;
pub mod schematic;
pub mod tech;
#[cfg(test)]
mod tests;
pub mod utils;
pub mod verilog;

pub use anyhow::{anyhow, Result};

pub const NETLIST_FORMAT: NetlistFormat = NetlistFormat::Spectre;
pub const BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");
#[cfg(feature = "calibre")]
pub const LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lib");

pub fn out_bin(name: &str) -> PathBuf {
    let mut path = PathBuf::from(BUILD_PATH);
    path.push(format!("pb/{}.pb.bin", name));
    path
}

pub fn save_bin(name: &str, pkg: Package) -> Result<()> {
    let input = SimInput {
        pkg: Some(pkg),
        top: name.to_string(),
        opts: None,
        an: vec![],
        ctrls: vec![],
    };

    let path = out_bin(name);
    std::fs::create_dir_all(path.parent().unwrap())?;
    vlsir::conv::save(&input, path).expect("Failed to save VLSIR data");

    Ok(())
}

pub fn generate_netlist(name: &str) -> Result<()> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("scripts/generate.py");

    let status = Command::new("python3")
        .args([path, name.into()])
        .stdout(Stdio::null())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "Netlist generation script failed with status {:?}",
            status.code()
        ))
    }
}

pub fn bbox(cell: &Ptr<Cell>) -> BoundBox {
    let cell = cell.read().unwrap();
    cell.layout.as_ref().unwrap().bbox()
}

#[inline]
pub(crate) fn clog2(x: usize) -> usize {
    (x as f64).log2().ceil() as usize
}
