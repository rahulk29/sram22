use pdkprims::tech::sky130;

use crate::config::{ControlMode, SramConfig};
use crate::layout::bank::{draw_sram_bank, SramBankParams};
use crate::schematic::sram::{sram, SramParams};
use crate::utils::save_modules;
use crate::verification::bit_signal::BitSignal;
use crate::verification::{self, source_files, PortClass, PortOrder, TbParams, TestCase};
use crate::verilog::{save_1rw_verilog, Sram1RwParams};
use crate::{clog2, generate_netlist, Result, BUILD_PATH};

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

pub(crate) fn test_gds_path(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(format!("gds/{}.gds", name))
}

pub(crate) fn test_verilog_path(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(format!("verilog/{}.v", name))
}

pub(crate) fn generate_test(config: SramConfig) -> Result<()> {
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

    let test_case = TestCase::builder()
        .clk_period(20e-9)
        .ops([
            verification::Op::Write {
                addr: BitSignal::from_u64(14, addr_width),
                data: BitSignal::from_u64(2321, data_width),
            },
            verification::Op::Read {
                addr: BitSignal::from_u64(14, addr_width),
            },
        ])
        .build()?;

    let mut ports = vec![
        (PortClass::Power, PortOrder::MsbFirst),
        (PortClass::Ground, PortOrder::MsbFirst),
        (PortClass::Clock, PortOrder::MsbFirst),
        (PortClass::DataIn, PortOrder::MsbFirst),
        (PortClass::DataOut, PortOrder::MsbFirst),
        (PortClass::WriteEnable, PortOrder::MsbFirst),
        (PortClass::Addr, PortOrder::MsbFirst),
    ];
    if wmask_groups > 1 {
        ports.push((PortClass::WriteMask, PortOrder::MsbFirst));
    }
    let mut tb = TbParams::builder();
    tb.test_case(test_case)
        .sram_name(&name)
        .tr(50e-12)
        .tf(50e-12)
        .vdd(1.8)
        .c_load(5e-15)
        .data_width(data_width)
        .addr_width(addr_width)
        .wmask_groups(wmask_groups)
        .ports(ports)
        .clk_port("clk")
        .write_enable_port("we")
        .addr_port("addr")
        .data_in_port("din")
        .data_out_port("dout")
        .pwr_port("vdd")
        .gnd_port("vss")
        .wmask_port("wmask")
        .work_dir(PathBuf::from(BUILD_PATH).join(format!("sim/{}", name)))
        .source_paths(source_files(&name));

    #[cfg(feature = "spectre")]
    tb.includes(crate::verification::spectre::sky130_includes());

    let tb = tb.build()?;

    verification::run_testbench(&tb)?;

    Ok(())
}
