use crate::backend::spice::SpiceBackend;
use crate::config::SramConfig;
use crate::error::Result;
use crate::predecode::predecode_3_8::PredecoderOptions;
use std::path::Path;

pub mod analysis;
pub mod backend;
pub mod cells;
pub mod config;
pub mod error;
pub mod predecode;

pub fn netlist(config: SramConfig) {
    std::fs::create_dir_all(&config.output_dir).unwrap();
    let mut b = SpiceBackend::new(Path::join(config.output_dir.as_ref(), "sram.spice"));
    emit_spice_prelude(&mut b).unwrap();

    let predecoder_opts = PredecoderOptions {
        power_net: "VPWR",
        gnd_net: "VGND",
    };
    predecode::predecode_3_8::netlist(&mut b, predecoder_opts).unwrap();
}

pub fn emit_spice_prelude(b: &mut SpiceBackend) -> Result<()> {
    b.title("SRAM 22 Memory")?;
    b.options("nopage")?;
    b.lib("/Users/rahul/acads/research/sky130/pdk/skywater-pdk/libraries/sky130_fd_pr/latest/models/sky130.lib.spice", "tt")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{netlist, SramConfig};

    #[test]
    fn test_generate() {
        let config = SramConfig {
            rows: 64,
            cols: 32,
            output_dir: "/tmp/sram22/tests/test_generate".to_string(),
        };
        netlist(config);
    }
}
