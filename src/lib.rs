use std::path::PathBuf;

use crate::verification::calibre::SKY130_LAYERPROPS_PATH;
pub use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
#[cfg(not(feature = "commercial"))]
use ngspice::Ngspice;
#[cfg(feature = "commercial")]
use sky130_commercial_pdk::Sky130CommercialPdk;
#[cfg(not(feature = "commercial"))]
use sky130_open_pdk::Sky130OpenPdk;
#[cfg(feature = "commercial")]
use spectre::Spectre;
#[cfg(feature = "commercial")]
use sub_calibre::CalibreDrc;
#[cfg(feature = "commercial")]
use sub_calibre::CalibreLvs;
#[cfg(feature = "commercial")]
use sub_calibre::CalibrePex;
use substrate::data::{SubstrateConfig, SubstrateCtx};
#[cfg(not(feature = "commercial"))]
use substrate::pdk::PdkParams;
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::verification::simulation::{Simulator, SimulatorOpts};
use tera::Tera;

#[cfg(feature = "commercial")]
pub mod abs;
pub mod blocks;
pub mod cli;
#[cfg(feature = "commercial")]
pub mod liberate;
pub mod measure;
pub mod paths;
pub mod pex;
pub mod plan;
pub mod tech;
pub mod verification;
pub mod verilog;

pub const BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");
pub const LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lib");
pub const SKY130_OPEN_PDK_ROOT: &str = env!("SKY130_OPEN_PDK_ROOT");
#[cfg(feature = "commercial")]
pub const SKY130_COMMERCIAL_PDK_ROOT: &str = env!("SKY130_COMMERCIAL_PDK_ROOT");

lazy_static! {
    pub static ref TEMPLATES: Tera =
        match Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/*")) {
            Ok(t) => t,
            Err(e) => panic!("Error parsing templates: {e}"),
        };
}

pub fn bus_bit(name: &str, index: usize) -> String {
    format!("{name}[{index}]")
}

#[inline]
pub(crate) fn clog2(x: usize) -> usize {
    (x as f64).log2().ceil() as usize
}

pub fn setup_ctx() -> SubstrateCtx {
    #[cfg(not(feature = "commercial"))]
    let simulator = Ngspice::new(SimulatorOpts::default()).unwrap();

    #[cfg(feature = "commercial")]
    let simulator = Spectre::new(SimulatorOpts::default()).unwrap();

    let mut builder = SubstrateConfig::builder();

    #[cfg(feature = "commercial")]
    let builder = builder
        .pdk(
            Sky130CommercialPdk::new(
                PathBuf::from(SKY130_COMMERCIAL_PDK_ROOT),
                PathBuf::from(SKY130_OPEN_PDK_ROOT),
            )
            .unwrap(),
        )
        .drc_tool(
            CalibreDrc::builder()
                .rules_file(PathBuf::from(
                    crate::verification::calibre::SKY130_DRC_RULES_PATH,
                ))
                .runset_file(PathBuf::from(
                    crate::verification::calibre::SKY130_DRC_RUNSET_PATH,
                ))
                .layerprops(PathBuf::from(SKY130_LAYERPROPS_PATH))
                .build()
                .unwrap(),
        )
        .lvs_tool(
            CalibreLvs::builder()
                .rules_file(PathBuf::from(
                    crate::verification::calibre::SKY130_LVS_RULES_PATH,
                ))
                .layerprops(PathBuf::from(SKY130_LAYERPROPS_PATH))
                .build()
                .unwrap(),
        )
        .pex_tool(CalibrePex::new(PathBuf::from(
            crate::verification::calibre::SKY130_PEX_RULES_PATH,
        )));
    #[cfg(not(feature = "commercial"))]
    let builder = builder.pdk(
        Sky130OpenPdk::new(&PdkParams {
            pdk_root: PathBuf::from(SKY130_OPEN_PDK_ROOT),
        })
        .unwrap(),
    );

    #[cfg(feature = "commercial")]
    builder.simulation_bashrc("/tools/B/rahulkumar/sky130/priv/drc/.bashrc");

    let cfg = builder
        .netlister(SpiceNetlister::new())
        .simulator(simulator)
        .build();

    SubstrateCtx::from_config(cfg).unwrap()
}

#[cfg(test)]
pub mod tests {
    use std::path::PathBuf;

    use super::BUILD_PATH;

    pub(crate) fn test_work_dir(name: &str) -> PathBuf {
        PathBuf::from(BUILD_PATH).join(name)
    }
}
