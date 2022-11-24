use crate::config::{sram::SramParams, SramConfig};
use crate::layout::sram::draw_sram;
use crate::paths::{out_bin, out_gds, out_verilog};
use crate::plan::extract::ExtractionResult;
use crate::schematic::sram::sram;
use crate::schematic::{generate_netlist, save_modules};
use crate::verilog::save_1rw_verilog;
use crate::{clog2, Result};
use anyhow::Context;
use pdkprims::tech::sky130;
use std::path::Path;

pub mod extract;

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
pub struct SramPlan {
    pub sram_params: SramParams,
}

pub fn generate_plan(
    _extraction_result: ExtractionResult,
    config: &SramConfig,
) -> Result<SramPlan> {
    let &SramConfig {
        num_words,
        data_width,
        mux_ratio,
        write_size,
        control,
    } = config;

    let name = format!("sramgen_sram_{data_width}x{num_words}m{mux_ratio}w{write_size}_simple");
    let rows = (num_words / mux_ratio) as usize;
    let cols = (data_width * mux_ratio) as usize;
    let row_bits = clog2(rows);
    let col_bits = clog2(cols);
    let col_select_bits = clog2(mux_ratio as usize);
    let wmask_width = (data_width / write_size) as usize;
    let mux_ratio = mux_ratio as usize;
    let num_words = num_words as usize;
    let data_width = data_width as usize;
    let addr_width = clog2(num_words);

    Ok(SramPlan {
        sram_params: SramParams {
            name,
            wmask_width,
            row_bits,
            col_bits,
            col_select_bits,
            rows,
            cols,
            mux_ratio,
            num_words,
            data_width,
            addr_width,
            control,
        },
    })
}

pub fn execute_plan(work_dir: impl AsRef<Path>, plan: &SramPlan) -> Result<()> {
    let modules = sram(&plan.sram_params);

    let name = &plan.sram_params.name;

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules).with_context(|| "Error saving netlist binaries")?;

    generate_netlist(&bin_path, &work_dir)
        .with_context(|| "Error converting netlists to SPICE format")?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sram(&mut lib, &plan.sram_params).with_context(|| "Error generating SRAM layout")?;

    let gds_path = out_gds(&work_dir, name);
    let verilog_path = out_verilog(&work_dir, name);

    lib.save_gds(&gds_path)
        .with_context(|| "Error saving SRAM GDS")?;

    save_1rw_verilog(&verilog_path, &plan.sram_params)
        .with_context(|| "Error generating or saving Verilog model")?;

    #[cfg(feature = "calibre")]
    crate::verification::calibre::run_sram_drc_lvs(&name)?;

    #[cfg(feature = "abstract_lef")]
    {
        let lef_path = out_lef(&work_dir, name);
        crate::abs::run_sram_abstract(&name, &lef_path, &gds_path, &verilog_path)?;
    }

    #[cfg(feature = "spectre")]
    {
        let alternating_bits =
            0b0101010101010101010101010101010101010101010101010101010101010101u64;
        let test_case = TestCase::builder()
            .clk_period(20e-9)
            .ops([
                verification::Op::Write {
                    addr: BitSignal::from_u64(alternating_bits, addr_width),
                    data: BitSignal::from_u64(alternating_bits, data_width),
                },
                verification::Op::Read {
                    addr: BitSignal::from_u64(alternating_bits, addr_width),
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
            .source_paths(source_files(&name, VerificationTask::SpectreSim));

        tb.includes(crate::verification::spectre::sky130_includes());

        let tb = tb.build()?;

        verification::run_testbench(&tb).with_context(|| "Error simulating testbench")?;
    }

    Ok(())
}
