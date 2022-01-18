use magic_vlsi::MagicInstanceBuilder;

use crate::backend::spice::SpiceBackend;
use crate::config::SramConfig;
use crate::error::Result;
use crate::predecode::predecode_3_8::PredecoderOptions;
use std::fs;
use std::path::Path;

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

pub fn generate_64x32(config: SramConfig) -> Result<()> {
    let out_dir = &config.output_dir;
    let cell_dir = &config.cell_dir;

    // copy prereq cells
    fs::remove_dir_all(out_dir).unwrap();
    fs::create_dir_all(out_dir).unwrap();
    copy_cells(cell_dir, out_dir);
    let mut magic = MagicInstanceBuilder::new()
        .cwd(out_dir)
        .tech("sky130A")
        .build()
        .unwrap();

    magic.enable_box()?;
    magic.getcell("sram_sp_cell")?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;
    let bbox = magic.box_values()?;
    println!("width = {}", bbox.width());
    magic.exec_one(&format!("copy east {}", bbox.width()))?;
    magic.sideways()?;
    magic.save("sram_64x32")?;

    Ok(())
}

fn copy_cells(cell_dir: impl AsRef<Path>, out_dir: impl AsRef<Path>) {
    for cell_name in ["sram_sp_cell.mag"] {
        std::fs::copy(
            cell_dir.as_ref().join(cell_name),
            out_dir.as_ref().join(cell_name),
        )
        .unwrap();
    }
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
            cell_dir: "".to_string(),
        };
        netlist(config);
    }
}
