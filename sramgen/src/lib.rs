pub use anyhow::{anyhow, Result};

#[cfg(feature = "abstract_lef")]
pub mod abs;
pub mod cli;
pub mod config;
pub mod layout;
pub mod paths;
pub mod plan;
pub mod schematic;
pub mod tech;
#[cfg(test)]
mod tests;
pub mod verification;
pub mod verilog;

pub const BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "build");
pub const LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lib");

pub fn bus_bit(name: &str, index: usize) -> String {
    format!("{name}[{index}]")
}

#[inline]
pub(crate) fn clog2(x: usize) -> usize {
    (x as f64).log2().ceil() as usize
}
