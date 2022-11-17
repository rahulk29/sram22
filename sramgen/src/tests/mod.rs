use pdkprims::tech::sky130;

use crate::config::{ControlMode, SramConfig};
use crate::layout::bank::{draw_sram_bank, SramBankParams};
use crate::schematic::sram::{sram, SramParams};
use crate::utils::save_modules;
use crate::verilog::{save_1rw_verilog, Sram1RwParams};
use crate::{clog2, generate_netlist, Result, BUILD_PATH};
use std::fmt::Debug;
use std::path::PathBuf;

mod bitcells;
mod col_inv;
mod control;
mod decoder;
mod dff;
mod dout_buffer;
mod gate;
mod guard_ring;
mod latch;
mod mux;
mod precharge;
mod rbl;
mod sense_amp;
mod sram;
mod tmc;
mod wl_driver;
mod wmask_control;

pub(crate) fn panic_on_err<E: Debug>(e: E) -> ! {
    println!("ERROR: {e:?}");
    panic!("ERROR: {e:?}");
}

pub(crate) fn test_gds_path(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(format!("gds/{}.gds", name))
}

pub(crate) fn test_verilog_path(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(format!("verilog/{}.v", name))
}

pub fn generate_test(config: SramConfig) -> Result<()> {
    let SramConfig {
        num_words,
        data_width,
        mux_ratio,
        write_size,
        control,
    } = config;
    assert_eq!(
        control,
        ControlMode::Simple,
        "Only `ControlMode::Simple` is supported at the moment."
    );
    assert_eq!(data_width % write_size, 0);
    let name = format!("sramgen_sram_{data_width}x{num_words}m{mux_ratio}w{write_size}_simple");

    let rows = (num_words / mux_ratio) as usize;
    let cols = (data_width * mux_ratio) as usize;
    let row_bits = clog2(rows);
    let col_bits = clog2(cols);
    let col_mask_bits = clog2(mux_ratio as usize);
    let wmask_groups = (data_width / write_size) as usize;
    let mux_ratio = mux_ratio as usize;
    let num_words = num_words as usize;
    let data_width = data_width as usize;
    let addr_width = clog2(num_words);

    let modules = sram(SramParams {
        name: name.clone(),
        row_bits,
        col_bits,
        col_mask_bits,
        wmask_groups,
    });

    save_modules(&name, modules)?;

    generate_netlist(&name)?;

    let mut lib = sky130::pdk_lib(&name)?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            name: name.clone(),
            rows,
            cols,
            mux_ratio,
            wmask_groups,
        },
    )?;

    lib.save_gds(test_gds_path(&name))?;

    save_1rw_verilog(
        test_verilog_path(&name),
        Sram1RwParams {
            module_name: name.clone(),
            num_words,
            data_width,
            addr_width,
        },
    )
    .unwrap();

    #[cfg(feature = "calibre")]
    self::sram::calibre::run_sram_drc_lvs(&name)?;

    Ok(())
}
