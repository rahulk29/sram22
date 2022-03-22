use config::TechConfig;

use factory::{Component, LayoutFile};

use magic_vlsi::units::{Distance, Vec2};
use magic_vlsi::{Direction, MagicInstance};

use crate::bitcells::{BitcellArray, BitcellArrayParams};
use crate::cells::gates::inv::dec::InvDec;
use crate::cells::gates::inv::single_height::{InvParams, InvPmSh};
use crate::cells::gates::nand::single_height::{Nand2Params, Nand2PmSh};
use crate::config::SramConfig;
use crate::error::Result;
use crate::factory::{Factory, FactoryConfig};
use crate::layout::bus::BusBuilder;
use crate::precharge::layout::PrechargeParams;
use crate::precharge::PrechargeSize;
use crate::predecode::Predecoder2_4;

use std::path::{Path, PathBuf};

use log::info;

pub mod bitcells;
pub mod cells;
pub mod config;
pub mod decode;
pub mod error;
pub mod factory;
pub mod layout;
pub mod precharge;
pub mod predecode;

pub struct Sram;

impl Component for Sram {
    type Params = ();
    fn schematic(
        _ctx: factory::BuildContext,
        _params: Self::Params,
    ) -> micro_hdl::context::ContextTree {
        todo!()
    }
    fn layout(
        mut ctx: factory::BuildContext,
        _params: Self::Params,
    ) -> crate::error::Result<factory::Layout> {
        generate_sram(ctx.magic, ctx.tc, ctx.name)?;
        ctx.layout_from_default_magic()
    }
}

pub fn generate(cwd: PathBuf, config: SramConfig) -> Result<()> {
    let rows = config.rows;
    let cols = config.cols;
    assert_eq!(rows % 4, 0, "number of sram rows must be divisible by 4");
    assert_eq!(cols % 8, 0, "number of sram columns must be divisible by 8");

    assert!(cwd.is_absolute());

    let mut cell_dir = cwd.clone();
    cell_dir.push(&config.tech_dir);
    let mut output_dir = cwd;
    output_dir.push(&config.output_dir);

    info!("generating {}x{} SRAM", rows, cols);
    info!("generated files will be placed in {:?}", &output_dir);
    info!("reading cells from {:?}", &cell_dir);

    let cfg = FactoryConfig::builder()
        .out_dir(output_dir)
        .work_dir("/tmp/sram22/scratch".into())
        .tech_config(sky130_config())
        .build()
        .unwrap();
    let mut factory = Factory::from_config(cfg)?;

    include_cells(&mut factory, cell_dir)?;
    info!("copied custom cells to output directory");

    info!("generating subcells");
    factory.generate_layout::<InvPmSh>(
        "inv_pm_sh_2",
        InvParams {
            nmos_width: Distance::from_nm(1_000),
            li: "li".to_string(),
            m1: "m1".to_string(),
            height: Distance::from_nm(1_580),
            fingers: 2,
        },
    )?;
    factory.generate_layout::<Nand2PmSh>(
        "nand2_pm_sh",
        Nand2Params {
            nmos_scale: Distance::from_nm(800),
            height: Distance::from_nm(1_580),
        },
    )?;
    factory.generate_layout::<InvDec>("inv_dec", ())?;
    factory.generate_layout::<Predecoder2_4>("predecoder2_4", ())?;
    factory.generate_layout::<crate::precharge::layout::Precharge>(
        "precharge",
        PrechargeParams {
            sizing: PrechargeSize {
                rail_pmos_width_nm: 1_000,
                pass_pmos_width_nm: 420,
                pmos_length_nm: 150,
            },
            width: Distance::from_nm(1_200),
        },
    )?;
    factory.generate_layout::<BitcellArray>(
        "bitcell_array",
        BitcellArrayParams {
            rows: config.rows,
            cols: config.cols,
        },
    )?;
    info!("finished generating subcells");

    factory.generate_layout::<Sram>("sram_top", ())?;
    info!("DONE: finished generating sram");

    Ok(())
}

fn generate_sram(magic: &mut MagicInstance, tc: &TechConfig, name: &str) -> Result<()> {
    let bitcell_bank = magic.load_layout_cell("bitcell_array")?;

    magic.load(name)?;
    magic.enable_box()?;
    magic.drc_off()?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    let bitcell_bank = magic.place_layout_cell(bitcell_bank, Vec2::zero())?;

    let _bus = BusBuilder::new()
        .width(8)
        .dir(Direction::Up)
        .tech_layer(tc, "m1")
        .allow_contact(tc, "ct", "li")
        .allow_contact(tc, "via1", "m2")
        .align_right(bitcell_bank.bbox().left_edge() - tc.layer("m1").space)
        .start(bitcell_bank.bbox().bottom_edge())
        .end(bitcell_bank.bbox().top_edge())
        .draw(magic)?;

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
    magic.save(name)?;

    Ok(())
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
    .find(|x| x.is_err())
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
