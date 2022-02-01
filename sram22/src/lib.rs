use magic_vlsi::units::Rect;
use magic_vlsi::{Direction, MagicInstanceBuilder};

use crate::config::SramConfig;
use crate::error::Result;
use std::fs;
use std::path::Path;

pub mod cells;
pub mod config;
pub mod error;
pub mod layout;
pub mod predecode;

pub fn generate(config: SramConfig) -> Result<()> {
    let rows = config.rows;
    let cols = config.cols;
    assert_eq!(rows % 4, 0);
    assert_eq!(cols % 4, 0);

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
    magic.scalegrid(1, 2)?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;
    magic.load("sram_2x2")?;
    magic.enable_box()?;
    magic.getcell("sram_sp_cell")?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;
    magic.identify("sram0")?;
    magic.sideways()?; // orient nwell facing outwards
    let bbox = magic.box_values()?;
    magic.copy_dir(Direction::Right, bbox.width())?;
    magic.sideways()?;
    magic.identify("sram1")?;

    magic.exec_one("select clear")?;
    magic.exec_one("select cell sram0")?;
    magic.exec_one("select more cell sram1")?;
    magic.copy_dir(Direction::Down, bbox.height())?;
    magic.upside_down()?;
    magic.save("sram_2x2")?;

    let cell_name = format!("sram_{}x{}", rows, cols);

    magic.load("rowend")?;
    magic.select_top_cell()?;
    let rowend_bbox = magic.select_bbox()?;

    magic.load("corner")?;
    magic.select_top_cell()?;
    let corner_bbox = magic.select_bbox()?;

    magic.load(&cell_name)?;
    magic.enable_box()?;

    // draw top row
    let mut bbox = magic.getcell("corner")?;
    let left = bbox.left_edge();
    magic.sideways()?;
    for i in 0..(cols as usize) {
        bbox = magic.place_cell("colend", bbox.lr())?;
        if i % 2 == 1 {
            magic.sideways()?;
        }
    }
    magic.place_cell("corner", bbox.lr())?;

    // draw rows
    for i in 0..(rows as usize) {
        bbox = Rect::ul_wh(
            left,
            bbox.bottom_edge(),
            rowend_bbox.width(),
            rowend_bbox.height(),
        );
        bbox = magic.place_cell("rowend", bbox.ll())?;
        magic.sideways()?;
        if i % 2 == 0 {
            magic.upside_down()?;
        }

        for j in 0..(cols as usize) {
            bbox = magic.place_cell("sram_sp_cell", bbox.lr())?;

            if i % 2 == 0 {
                magic.upside_down()?;
            }

            if j % 2 == 0 {
                magic.sideways()?;
            }
        }
        magic.place_cell("rowend", bbox.lr())?;

        if i % 2 == 0 {
            magic.upside_down()?;
        }
    }

    // draw bot row
    bbox = Rect::ul_wh(
        left,
        bbox.bottom_edge(),
        corner_bbox.width(),
        corner_bbox.height(),
    );
    let mut bbox = magic.place_cell("corner", bbox.ll())?;
    magic.sideways()?;
    magic.upside_down()?;
    for i in 0..(cols as usize) {
        bbox = magic.place_cell("colend", bbox.lr())?;
        magic.upside_down()?;
        if i % 2 == 1 {
            magic.sideways()?;
        }
    }
    magic.place_cell("corner", bbox.lr())?;
    magic.upside_down()?;

    magic.save(&cell_name)?;

    Ok(())
}

fn copy_cells(cell_dir: impl AsRef<Path>, out_dir: impl AsRef<Path>) {
    for cell_name in ["sram_sp_cell.mag", "rowend.mag", "colend.mag", "corner.mag"] {
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

    use magic_vlsi::{MagicInstance, MagicInstanceBuilder};

    use crate::config::TechConfig;

    static COUNTER: AtomicU64 = AtomicU64::new(1);

    pub fn id() -> u64 {
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn tmpdir() -> PathBuf {
        let id = id();
        let path = PathBuf::from(format!("/tmp/sram22/tests/{}", id));
        std::fs::create_dir_all(&path).expect("failed to create temp directory for testing");
        path
    }

    pub fn sky130_config() -> TechConfig {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tech/sky130/drc_config.toml");
        TechConfig::load(p).expect("failed to load sky130A tech config")
    }

    pub fn get_magic() -> MagicInstance {
        let dir = tmpdir();
        let id = id();
        let port = id + 8_000;
        MagicInstanceBuilder::new()
            .port(port as u16)
            .cwd(dir)
            .tech("sky130A")
            .build()
            .expect("failed to start magic")
    }
}
