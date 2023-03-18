use std::path::PathBuf;
use std::sync::Arc;

pub use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use ngspice::Ngspice;
#[cfg(feature = "commercial")]
use sky130_commercial_pdk::Sky130CommercialPdk;
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
use substrate::pdk::{Pdk, PdkParams};
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::verification::simulation::{Simulator, SimulatorOpts};
use tera::Tera;

#[cfg(feature = "commercial")]
pub mod abs;
pub mod cli;
pub mod config;
pub mod layout;
#[cfg(feature = "commercial")]
pub mod liberate;
pub mod paths;
pub mod plan;
pub mod schematic;
pub mod tech;
#[cfg(test)]
mod tests;
pub mod v2;
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

#[inline]
fn into_map<T, U>(v: Vec<T>) -> Vec<U>
where
    T: Into<U>,
{
    v.into_iter().map(|x| x.into()).collect()
}

pub fn setup_ctx() -> SubstrateCtx {
    #[cfg(not(feature = "commercial"))]
    let simulator = Ngspice::new(SimulatorOpts::default()).unwrap();

    #[cfg(feature = "commercial")]
    let simulator = Spectre::new(SimulatorOpts::default()).unwrap();

    let builder = SubstrateConfig::builder();

    #[cfg(feature = "commercial")]
    let builder = builder
        .pdk(Arc::new(
            Sky130CommercialPdk::new(&PdkParams {
                pdk_root: PathBuf::from(SKY130_COMMERCIAL_PDK_ROOT),
            })
            .unwrap(),
        ))
        .drc_tool(Arc::new(
            CalibreDrc::builder()
                .rules_file(PathBuf::from(
                    crate::verification::calibre::SKY130_DRC_RULES_PATH,
                ))
                .runset_file(PathBuf::from(
                    crate::verification::calibre::SKY130_DRC_RUNSET_PATH,
                ))
                .build()
                .unwrap(),
        ))
        .lvs_tool(Arc::new(CalibreLvs::new(PathBuf::from(
            crate::verification::calibre::SKY130_LVS_RULES_PATH,
        ))))
        .pex_tool(Arc::new(CalibrePex::new(PathBuf::from(
            crate::verification::calibre::SKY130_PEX_RULES_PATH,
        ))));
    #[cfg(not(feature = "commercial"))]
    let builder = builder.pdk(Arc::new(
        Sky130OpenPdk::new(&PdkParams {
            pdk_root: PathBuf::from(SKY130_OPEN_PDK_ROOT),
        })
        .unwrap(),
    ));

    let cfg = builder
        .netlister(Arc::new(SpiceNetlister::new()))
        .simulator(Arc::new(simulator))
        .build()
        .unwrap();
    SubstrateCtx::from_config(cfg)
}
