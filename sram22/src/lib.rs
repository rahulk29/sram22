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
    let rows = 32;
    let cols = 64;
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

    crate::cells::gates::inv::generate_pm(&mut magic)?;
    crate::cells::gates::inv::generate_pm_eo(&mut magic)?;

    magic.drc_off()?;
    magic.load("sram_2x2")?;
    magic.enable_box()?;
    magic.getcell("sram_cell_wired")?;
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
    magic.save("sram_2x2")?;

    let cell_name = format!("sram_{}x{}", rows, cols);

    magic.load(&cell_name)?;
    magic.enable_box()?;
    magic.getcell("sram_2x2")?;
    magic.array(cols / 2, rows / 2)?;
    magic.save(&cell_name)?;

    Ok(())
}

fn copy_cells(cell_dir: impl AsRef<Path>, out_dir: impl AsRef<Path>) {
    for cell_name in ["sram_sp_cell.mag", "sram_cell_wired.mag", "inv4.mag"] {
        std::fs::copy(
            cell_dir.as_ref().join(cell_name),
            out_dir.as_ref().join(cell_name),
        )
        .unwrap();
    }
}

#[cfg(test)]
mod tests {}

#[cfg(test)]
pub(crate) mod test_utils {
    use std::{path::PathBuf, sync::atomic::AtomicU64};

    static COUNTER: AtomicU64 = AtomicU64::new(1);

    pub fn id() -> u64 {
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn tmpdir() -> PathBuf {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = PathBuf::from(format!("/tmp/sram22/tests/{}", id));
        std::fs::create_dir_all(&path).expect("failed to create temp directory for testing");
        path
    }
}
