use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::anyhow;
use vlsir::circuit::{port, ExternalModule, Package, Port};
use vlsir::reference::To;
use vlsir::spice::SimInput;
use vlsir::{Module, QualifiedName, Reference};

use crate::schematic::vlsir_api::{port, signal};
use crate::tech::all_external_modules;
use crate::Result;

pub mod bitcell_array;
pub mod col_inv;
pub mod decoder;
pub mod dff;
pub mod dout_buffer;
pub mod edge_detector;
pub mod gate;
pub mod inv_chain;
pub mod latch;
pub mod mos;
pub mod mux;
pub mod precharge;
pub mod sense_amp;
pub mod sram;
pub mod wl_driver;
pub mod wmask_control;

pub mod vlsir_api;

pub const GENERATE_SCRIPT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/generate.py");
pub const NETLIST_FORMAT: NetlistFormat = NetlistFormat::Spectre;

pub enum NetlistFormat {
    NgSpice,
    Spectre,
}

pub fn simple_ext_module(
    domain: impl Into<String>,
    name: impl Into<String>,
    ports: &[&str],
) -> ExternalModule {
    let ports = ports
        .iter()
        .map(|&n| port(signal(n), port::Direction::Inout))
        .collect::<Vec<_>>();

    ExternalModule {
        name: Some(QualifiedName {
            domain: domain.into(),
            name: name.into(),
        }),
        desc: "An external module".to_string(),
        ports,
        parameters: vec![],
    }
}

pub fn save_modules(path: impl AsRef<Path>, name: &str, modules: Vec<Module>) -> Result<()> {
    let ext_modules = all_external_modules();
    let pkg = vlsir::circuit::Package {
        domain: format!("sramgen_{}", name),
        desc: "Sramgen generated cells".to_string(),
        modules,
        ext_modules,
    };

    save_bin(path, name, pkg)?;

    Ok(())
}

pub fn save_bin(path: impl AsRef<Path>, name: &str, pkg: Package) -> Result<()> {
    let input = SimInput {
        pkg: Some(pkg),
        top: name.to_string(),
        opts: None,
        an: vec![],
        ctrls: vec![],
    };

    std::fs::create_dir_all(path.as_ref().parent().unwrap())?;
    vlsir::conv::save(&input, path).expect("Failed to save VLSIR data");

    Ok(())
}

pub fn generate_netlist(bin_path: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> Result<()> {
    let status = Command::new("python3")
        .arg(GENERATE_SCRIPT_PATH)
        .arg(bin_path.as_ref())
        .arg("-o")
        .arg(output_dir.as_ref())
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
