use std::path::PathBuf;

use decoder::DecoderTree;
use layout21::raw::{BoundBox, Cell};
use layout21::utils::Ptr;
use mos::NetlistFormat;
use vlsir::circuit::Package;
use vlsir::spice::SimInput;

pub mod bitcells;
pub mod col_inv;
pub mod config;
pub mod decoder;
pub mod dff;
pub mod gate;
pub mod layout;
pub mod mos;
pub mod mux;
pub mod netlist;
pub mod plan;
pub mod precharge;
pub mod rbl;
pub mod sense_amp;
pub mod sram;
pub mod tech;
pub mod utils;
pub mod wl_driver;
pub mod wmask_control;
pub mod write_driver;

pub use anyhow::Result;

pub const NETLIST_FORMAT: NetlistFormat = NetlistFormat::Spectre;

pub fn generate() -> Result<()> {
    let nmos = mos::ext_nmos(NETLIST_FORMAT);
    let pmos = mos::ext_pmos(NETLIST_FORMAT);

    let pkg = Package {
        domain: "sramgen".to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![],
        ext_modules: vec![nmos, pmos],
    };

    let _decoder = DecoderTree::new(5);

    let input = SimInput {
        pkg: Some(pkg),
        top: "nand2".to_string(),
        opts: None,
        an: vec![],
        ctrls: vec![],
    };

    vlsir::conv::save(&input, "hi.bin").expect("Failed to save VLSIR data");

    Ok(())
}

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
