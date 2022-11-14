use std::path::PathBuf;

use layout21::raw::{BoundBox, Cell};
use layout21::utils::Ptr;
use schematic::mos::NetlistFormat;
use vlsir::circuit::Package;
use vlsir::spice::SimInput;

pub mod cli;
pub mod config;
pub mod layout;
pub mod plan;
pub mod schematic;
pub mod tech;
pub mod utils;
pub mod verilog;

pub use anyhow::Result;

pub const NETLIST_FORMAT: NetlistFormat = NetlistFormat::Spectre;

pub fn out_bin(name: &str) -> PathBuf {
    format!("build/pb/{}.pb.bin", name).into()
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

pub fn bbox(cell: &Ptr<Cell>) -> BoundBox {
    let cell = cell.read().unwrap();
    cell.layout.as_ref().unwrap().bbox()
}

#[inline]
pub(crate) fn clog2(x: usize) -> usize {
    (x as f64).log2().ceil() as usize
}
