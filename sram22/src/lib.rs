use config::TechConfig;
use indicatif::{ProgressBar, ProgressStyle};
use magic_vlsi::units::{Distance, Rect, Vec2};
use magic_vlsi::{Direction, MagicInstance, MagicInstanceBuilder};

use crate::cells::gates::inv::single_height::InvParams;
use crate::cells::gates::nand::single_height::Nand2Params;
use crate::config::SramConfig;
use crate::error::Result;
use crate::layout::bus::BusBuilder;
use std::fs;
use std::path::{Path, PathBuf};

use log::info;

pub mod cells;
pub mod config;
pub mod error;
pub mod layout;
pub mod predecode;

pub fn generate(config: SramConfig) -> Result<()> {
    let rows = config.rows;
    let cols = config.cols;
    assert_eq!(rows % 4, 0, "number of sram rows must be divisible by 4");
    assert_eq!(cols % 4, 0, "number of sram columns must be divisible by 4");

    info!("generating {}x{} SRAM", rows, cols);
    info!("generated files will be placed in {}", &config.output_dir);
    info!("reading cells from {}", &config.tech_dir);

    let out_dir = &config.output_dir;
    let cell_dir = &config.tech_dir;

    // clean the existing build directory; ignore errors
    let _ = fs::remove_dir_all(out_dir);

    // copy prereq cells
    fs::create_dir_all(out_dir).unwrap();
    copy_cells(cell_dir, out_dir);
    info!("copied custom cells to output directory");

    let tc = sky130_config();

    let mut magic = MagicInstanceBuilder::new()
        .cwd(out_dir)
        .tech("sky130A")
        .build()
        .unwrap();
    magic.drc_off()?;
    magic.scalegrid(1, 2)?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    info!("magic started successfully");

    info!("generating subcells");
    crate::cells::gates::inv::generate_pm(&mut magic)?;
    crate::cells::gates::inv::generate_pm_eo(&mut magic)?;
    crate::cells::gates::inv::single_height::generate_pm_single_height(
        &mut magic,
        &tc,
        &InvParams {
            nmos_width: Distance::from_nm(1_000),
            li: "li".to_string(),
            m1: "m1".to_string(),
            height: Distance::from_nm(1_580),
            fingers: 2,
        },
    )?;
    crate::cells::gates::nand::single_height::generate_pm_single_height(
        &mut magic,
        &tc,
        &Nand2Params {
            nmos_scale: Distance::from_nm(800),
            height: Distance::from_nm(1_580),
        },
    )?;
    crate::cells::gates::inv::dec::generate_inv_dec(&mut magic, &tc)?;
    crate::predecode::generate_predecoder2_4(&mut magic, &tc)?;
    info!("finished generating subcells");

    let bitcell_name = generate_bitcell_array(&mut magic, &tc, &config)?;

    let bitcell_bank = magic.load_layout_cell(&bitcell_name)?;

    let cell_name = format!("sram_{}x{}", rows, cols);
    magic.load(&cell_name)?;
    magic.enable_box()?;
    magic.drc_off()?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    let bitcell_bank = magic.place_layout_cell(bitcell_bank, Vec2::zero())?;

    let bus = BusBuilder::new()
        .width(8)
        .dir(Direction::Up)
        .tech_layer(&tc, "m1")
        .allow_contact(&tc, "ct", "li")
        .allow_contact(&tc, "via1", "m2")
        .align_right(bitcell_bank.bbox().left_edge() - tc.layer("m1").space)
        .start(bitcell_bank.bbox().bottom_edge())
        .end(bitcell_bank.bbox().top_edge())
        .draw(&mut magic)?;

    for i in 0..4 {
        for j in 0..4 {
            let nand_in1 = bitcell_bank.port_bbox(&format!("wl_{}A", 4 * i + j));
            bus.draw_contact(&mut magic, &tc, i, "ct", "viali", "li", nand_in1)?;
            let nand_in2 = bitcell_bank.port_bbox(&format!("wl_{}B", 4 * i + j));
            bus.draw_contact(&mut magic, &tc, 4 + j, "ct", "viali", "li", nand_in2)?;
        }
    }

    info!("generated bus for predecoder outputs");

    info!("layout complete; saving sram cell");
    magic.save(&cell_name)?;

    info!("DONE: finished generating sram");

    Ok(())
}

fn generate_bitcell_array(
    magic: &mut MagicInstance,
    _tc: &TechConfig,
    config: &SramConfig,
) -> Result<String> {
    let rows = config.rows;
    let cols = config.cols;
    let cell_name = format!("bitcells_{}x{}", rows, cols);

    let rowend = magic.load_layout_cell("rowend")?;
    let inv_dec = magic.load_layout_cell("inv_dec_auto")?;
    let nand2_dec = magic.load_layout_cell("nand2_dec_auto")?;
    let corner = magic.load_layout_cell("corner")?;

    magic.load(&cell_name)?;
    magic.enable_box()?;
    magic.drc_off()?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    info!("generating bitcell array");

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
    info!("generated top row");

    // draw rows
    info!("generating bitcell core");
    let prog_bar = ProgressBar::new((rows * cols) as u64);
    prog_bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar}] {pos:>4}/{len:4} [{eta_precise}]")
            .progress_chars("=> "),
    );
    prog_bar.set_message("Placing sram cells");
    for i in 0..(rows as usize) {
        let pre_column_dist = inv_dec.bbox.width() + nand2_dec.bbox.width();
        let sram_array_left = left - pre_column_dist;
        bbox = Rect::ul_wh(
            sram_array_left,
            bbox.bottom_edge(),
            pre_column_dist,
            rowend.bbox.height(),
        );
        let mut nand2_cell = magic.place_layout_cell(nand2_dec.clone(), bbox.ll())?;
        if i % 2 == 0 {
            magic.flip_cell_y(&mut nand2_cell)?;
        }
        magic.rename_cell_pin(&nand2_cell, "A", &format!("wl_{}A", i))?;
        magic.port_make_default()?;
        magic.rename_cell_pin(&nand2_cell, "B", &format!("wl_{}B", i))?;
        magic.port_make_default()?;
        bbox = nand2_cell.bbox();
        bbox = magic.place_cell("inv_dec_auto", bbox.lr())?;
        if i % 2 == 0 {
            magic.upside_down()?;
        }
        bbox = magic.place_cell("rowend", bbox.lr())?;
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
            prog_bar.inc(1);
        }
        magic.place_cell("rowend", bbox.lr())?;

        if i % 2 == 0 {
            magic.upside_down()?;
        }
    }

    prog_bar.finish_and_clear();

    info!("finished generating bitcell core");

    // draw bot row
    bbox = Rect::ul_wh(
        left,
        bbox.bottom_edge(),
        corner.bbox.width(),
        corner.bbox.height(),
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

    info!("generated bottom row");

    magic.port_renumber()?;
    magic.save(&cell_name)?;

    Ok(cell_name)
}

fn copy_cells(cell_dir: impl AsRef<Path>, out_dir: impl AsRef<Path>) {
    for cell_name in [
        "sram_sp_cell.mag",
        "rowend.mag",
        "colend.mag",
        "corner.mag",
        "wl_route.mag",
        "inv_dec.mag",
        "nand2_dec.mag",
        "wlstrap.mag",
        "wlstrap_p.mag",
        "colend_cent.mag",
        "colend_p_cent.mag",
    ] {
        std::fs::copy(
            cell_dir.as_ref().join(cell_name),
            out_dir.as_ref().join(cell_name),
        )
        .unwrap();
    }
}

pub fn sky130_config() -> TechConfig {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../tech/sky130/drc_config.toml");
    TechConfig::load(p).expect("failed to load sky130A tech config")
}

pub fn net_name_bar(prefix: &str, bar: bool) -> String {
    if bar {
        format!("{}b", prefix)
    } else {
        prefix.into()
    }
}

#[cfg(test)]
mod tests {}

#[cfg(test)]
pub(crate) mod test_utils {
    use std::{path::PathBuf, sync::atomic::AtomicU64};

    use magic_vlsi::{MagicInstance, MagicInstanceBuilder};

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
