use magic_vlsi::{Direction, MagicInstanceBuilder};

use crate::config::SramConfig;
use crate::error::Result;
use std::fs;
use std::path::Path;

pub mod cells;
pub mod config;
pub mod error;
pub mod predecode;

pub fn generate_32x64(config: SramConfig) -> Result<()> {
    let out_dir = &config.output_dir;
    let cell_dir = &config.cell_dir;

    // clean the existing build directory; ignore errors
    let _ = fs::remove_dir_all(out_dir);

    // copy prereq cells
    fs::create_dir_all(out_dir).unwrap();
    copy_cells(cell_dir, out_dir);
    let mut magic = MagicInstanceBuilder::new()
        .cwd(out_dir)
        .tech("sky130A")
        .build()
        .unwrap();

    magic.drc_off()?;
    magic.load("sram_4x4")?;
    magic.enable_box()?;
    magic.getcell("sram_sp_cell")?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;
    magic.identify("sram0")?;
    let bbox = magic.box_values()?;
    magic.copy_dir(Direction::Right, bbox.width())?;
    // magic.exec_one(&format!("copy east {}", bbox.width()))?;
    magic.sideways()?;
    magic.identify("sram1")?;

    magic.exec_one("select clear")?;
    magic.exec_one("select cell sram0")?;
    magic.exec_one("select more cell sram1")?;
    magic.copy_dir(Direction::Down, bbox.height())?;
    magic.upside_down()?;
    magic.save("sram_4x4")?;

    magic.load("sram_32x64")?;
    magic.enable_box()?;
    magic.getcell("sram_4x4")?;
    magic.array(16, 8)?;
    magic.save("sram_32x64")?;

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
mod tests {}
