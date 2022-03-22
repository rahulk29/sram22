use config::TechConfig;

use factory::LayoutFile;
use layout::grid::{GridCell, GridLayout};

use magic_vlsi::units::{Distance, Vec2};
use magic_vlsi::{Direction, MagicInstance};

use crate::cells::gates::inv::single_height::InvParams;
use crate::cells::gates::nand::single_height::Nand2Params;
use crate::config::SramConfig;
use crate::error::Result;
use crate::factory::{Factory, FactoryConfig};
use crate::layout::bus::BusBuilder;
use crate::precharge::PrechargeSize;
use std::fs;
use std::path::{Path, PathBuf};

use log::info;

pub mod cells;
pub mod config;
pub mod decode;
pub mod error;
pub mod factory;
pub mod layout;
pub mod precharge;
pub mod predecode;

pub fn generate(config: SramConfig) -> Result<()> {
    let rows = config.rows;
    let cols = config.cols;
    assert_eq!(rows % 4, 0, "number of sram rows must be divisible by 4");
    assert_eq!(cols % 8, 0, "number of sram columns must be divisible by 8");

    info!("generating {}x{} SRAM", rows, cols);
    info!("generated files will be placed in {}", &config.output_dir);
    info!("reading cells from {}", &config.tech_dir);

    let cell_dir = config.tech_dir.clone();

    let cfg = FactoryConfig::builder()
        .out_dir(config.output_dir.into())
        .work_dir("/tmp/sram22/scratch".into())
        .tech_config(sky130_config())
        .build()
        .unwrap();
    let mut factory = Factory::from_config(cfg)?;

    include_cells(&mut factory, cell_dir);
    info!("copied custom cells to output directory");

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
    crate::precharge::layout::generate_precharge(
        &mut magic,
        &tc,
        PrechargeSize {
            rail_pmos_width_nm: 1_000,
            pass_pmos_width_nm: 420,
            pmos_length_nm: 150,
        },
        Distance::from_nm(1_200),
    )?;
    info!("finished generating subcells");

    let bitcell_name = generate_bitcells(&mut magic, &config)?;

    let bitcell_bank = magic.load_layout_cell(&bitcell_name)?;

    let cell_name = format!("sram_{}x{}", rows, cols);
    magic.load(&cell_name)?;
    magic.enable_box()?;
    magic.drc_off()?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    let bitcell_bank = magic.place_layout_cell(bitcell_bank, Vec2::zero())?;

    let _bus = BusBuilder::new()
        .width(8)
        .dir(Direction::Up)
        .tech_layer(&tc, "m1")
        .allow_contact(&tc, "ct", "li")
        .allow_contact(&tc, "via1", "m2")
        .align_right(bitcell_bank.bbox().left_edge() - tc.layer("m1").space)
        .start(bitcell_bank.bbox().bottom_edge())
        .end(bitcell_bank.bbox().top_edge())
        .draw(&mut magic)?;

    for _i in 0..4 {
        for _j in 0..4 {
            // let nand_in1 = bitcell_bank.port_bbox(&format!("wl_{}A", 4 * i + j));
            // bus.draw_contact(&mut magic, &tc, i, "ct", "viali", "li", nand_in1)?;
            // let nand_in2 = bitcell_bank.port_bbox(&format!("wl_{}B", 4 * i + j));
            // bus.draw_contact(&mut magic, &tc, 4 + j, "ct", "viali", "li", nand_in2)?;
        }
    }

    info!("generated bus for predecoder outputs");

    info!("layout complete; saving sram cell");
    magic.save(&cell_name)?;

    info!("DONE: finished generating sram");

    Ok(())
}

fn plan_bitcell_array(
    magic: &mut MagicInstance,
    config: &SramConfig,
) -> Result<grid::Grid<Option<GridCell>>> {
    let rows = config.rows as usize;

    let top_row = plan_colend_row(magic, config, false)?;

    let bitcell_rows: Result<Vec<Vec<Option<GridCell>>>> = (0..rows as usize)
        .map(|i| {
            info!("planning bitcell row {}", i + 1);
            plan_bitcell_row(magic, config, i)
        })
        .collect();
    let bitcell_rows = bitcell_rows?;

    let bot_row = plan_colend_row(magic, config, true)?;
    let mut grid: grid::Grid<Option<GridCell>> = grid::grid![];
    grid.push_row(top_row);

    for row in bitcell_rows {
        grid.push_row(row);
    }

    grid.push_row(bot_row);

    Ok(grid)
}

fn plan_colend_row(
    magic: &mut MagicInstance,
    config: &SramConfig,
    bottom: bool,
) -> Result<Vec<Option<GridCell>>> {
    let corner = magic.load_layout_cell("corner")?;
    let colend = magic.load_layout_cell("colend")?;
    let colend_cent = magic.load_layout_cell("colend_cent")?;

    // 2 slots for decoder gates
    let mut top_row = vec![
        None,
        None,
        Some(GridCell::new(corner.clone(), true, bottom)),
    ];

    for i in 0..config.cols as usize {
        if i > 0 && i % 8 == 0 {
            top_row.push(Some(GridCell::new(colend_cent.clone(), i % 2 != 0, bottom)));
        }
        top_row.push(Some(GridCell::new(colend.clone(), i % 2 != 0, bottom)));
    }

    top_row.push(Some(GridCell::new(corner, false, bottom)));

    info!("generated {} row cells", top_row.len());

    Ok(top_row)
}

fn plan_bitcell_row(
    magic: &mut MagicInstance,
    config: &SramConfig,
    idx: usize,
) -> Result<Vec<Option<GridCell>>> {
    let rowend = magic.load_layout_cell("rowend")?;
    let bitcell = magic.load_layout_cell("sram_sp_cell")?;
    let nand2_dec = magic.load_layout_cell("nand2_dec_auto")?;
    let inv_dec = magic.load_layout_cell("inv_dec_auto")?;
    let wlstrap = magic.load_layout_cell("wlstrap")?;

    let mut row = Vec::new();
    let flip_y = idx % 2 == 0;

    row.push(Some(GridCell::new(nand2_dec, false, flip_y)));
    row.push(Some(GridCell::new(inv_dec, false, flip_y)));
    row.push(Some(GridCell::new(rowend.clone(), true, flip_y)));

    for i in 0..config.cols as usize {
        if i > 0 && i % 8 == 0 {
            row.push(Some(GridCell::new(wlstrap.clone(), false, flip_y)));
        }
        row.push(Some(GridCell::new(bitcell.clone(), i % 2 == 0, flip_y)));
    }

    row.push(Some(GridCell::new(rowend, false, flip_y)));

    Ok(row)
}

fn generate_bitcells(magic: &mut MagicInstance, config: &SramConfig) -> Result<String> {
    info!("generating bitcell array");
    let cell_name = format!("bitcells_{}x{}", config.rows, config.cols);

    let grid = plan_bitcell_array(magic, config)?;

    magic.load(&cell_name)?;
    magic.enable_box()?;
    magic.drc_off()?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    let grid = GridLayout::new(grid);
    grid.draw(magic, Vec2::zero())?;

    magic.port_renumber()?;
    magic.save(&cell_name)?;
    magic.exec_one("writeall force")?;

    info!("saved {}", &cell_name);
    Ok(cell_name)
}

fn include_cells(factory: &mut Factory, cell_dir: impl AsRef<Path>) -> Result<()> {
    [
        "sram_sp_cell",
        "rowend",
        "colend",
        "corner",
        "wl_route",
        "inv_dec",
        "nand2_dec",
        "wlstrap",
        "wlstrap_p",
        "colend_cent",
        "colend_p_cent",
        "sa_senseamp",
    ]
    .iter()
    .map(|cell_name| {
        let path = cell_dir.as_ref().join(&format!("{}.mag", cell_name));
        factory.include_layout(cell_name, LayoutFile::Magic(path))?;
        Ok(())
    })
    .filter(|x| x.is_err())
    .next()
    .unwrap_or(Ok(()))
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

pub fn clog2(mut x: usize) -> u8 {
    assert!(x > 0, "clog2: cannot take log of 0");
    let mut ctr = 0u8;
    while x > 1 {
        x >>= 1;
        ctr += 1;
    }

    ctr
}

pub fn tech_spice_include() -> PathBuf {
    "/home/rahul/acads/sky130/skywater-pdk/libraries/sky130_fd_pr/latest/models/sky130.lib.spice"
        .into()
}

#[cfg(test)]
mod tests {}

#[cfg(test)]
pub(crate) mod test_utils {
    use std::{path::PathBuf, sync::atomic::AtomicU64};

    use magic_vlsi::MagicInstance;

    static COUNTER: AtomicU64 = AtomicU64::new(1);

    pub fn id() -> u64 {
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn get_port() -> u16 {
        (id() + 8_000) as u16
    }

    pub fn tmpdir() -> PathBuf {
        let id = id();
        let path = PathBuf::from(format!("/tmp/sram22/tests/{}", id));
        std::fs::create_dir_all(&path).expect("failed to create temp directory for testing");
        path
    }

    pub fn get_magic() -> MagicInstance {
        let dir = tmpdir();
        let port = get_port();
        MagicInstance::builder()
            .port(port as u16)
            .cwd(dir)
            .tech("sky130A")
            .build()
            .expect("failed to start magic")
    }
}
