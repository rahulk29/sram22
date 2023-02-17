use std::path::PathBuf;
use std::sync::Arc;

pub use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use ngspice::Ngspice;
use sky130_open_pdk::Sky130Pdk;
#[cfg(feature = "calibre")]
use sub_calibre::CalibreDrc;
#[cfg(feature = "calibre")]
use sub_calibre::CalibreLvs;
#[cfg(feature = "calibre")]
use sub_calibre::CalibrePex;
use substrate::data::{SubstrateConfig, SubstrateCtx};
use substrate::pdk::{Pdk, PdkParams};
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::verification::simulation::{Simulator, SimulatorOpts};
use tera::Tera;

#[cfg(feature = "abstract_lef")]
pub mod abs;
pub mod cli;
pub mod config;
pub mod layout;
#[cfg(feature = "liberate_mx")]
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
    let simulator = Ngspice::new(SimulatorOpts::default()).unwrap();

    let builder = SubstrateConfig::builder();

    #[cfg(feature = "calibre")]
    let builder = builder
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
    let cfg = builder
        .netlister(Arc::new(SpiceNetlister::new()))
        .simulator(Arc::new(simulator))
        .pdk(Arc::new(
            Sky130Pdk::new(&PdkParams {
                pdk_root: PathBuf::from("/Users/rahul/acads/research/sky130/pdk/skywater-pdk"),
            })
            .unwrap(),
        ))
        .build()
        .unwrap();
    SubstrateCtx::from_config(cfg)
}
