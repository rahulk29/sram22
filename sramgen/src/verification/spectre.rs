use anyhow::{bail, Context};
use psf_ascii::parser::transient::TransientData;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::sram::SramParams;
use crate::verification::bit_signal::BitSignal;
use crate::verification::{
    self, source_files, PortClass, PortOrder, TbParams, TestCase, VerificationTask,
};
use crate::Result;

pub struct SpectreParams {
    pub work_dir: PathBuf,
    pub spice_path: PathBuf,
}

pub struct SpectreGeneratedPaths {
    pub raw_output_dir: PathBuf,
    pub log_path: PathBuf,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
}

pub fn sky130_includes() -> Vec<String> {
    vec![
        "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/MODELS/SPECTRE/s8x/Models/models.all".into(),
        "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/MODELS/SPECTRE/s8x/Models/tt.cor".into(),
        "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/MODELS/SPECTRE/s8x/Models/ttcell.cor".into(),
        "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/MODELS/SPECTRE/s8x/Models/npass.pm3".into(),
        "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/MODELS/SPECTRE/s8x/Models/npd.pm3".into(),
        "/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/MODELS/SPECTRE/s8x/Models/ppu.pm3".into(),
    ]
}

pub fn run_spectre(params: &SpectreParams) -> Result<TransientData> {
    let paths = generate_paths(params);

    let out_file = std::fs::File::create(paths.stdout_path)?;
    let err_file = std::fs::File::create(paths.stderr_path)?;

    let status = Command::new("spectre")
        .arg("-64")
        .arg("+spice")
        .arg("+aps")
        .arg("-format")
        .arg("psfascii")
        .arg(&params.spice_path)
        .arg("-raw")
        .arg(&paths.raw_output_dir)
        .arg("=log")
        .arg(&paths.log_path)
        .stdout(out_file)
        .stderr(err_file)
        .current_dir(&params.work_dir)
        .status()?;

    if !status.success() {
        bail!("Spectre exited unsuccessfully");
    }

    // Spectre chooses this file name by default
    let psf_path = paths.raw_output_dir.join("timeSweep.tran.tran");
    let psf = std::fs::read_to_string(psf_path)?;
    let ast = psf_ascii::parser::frontend::parse(&psf)?;
    let data = TransientData::from_ast(&ast);

    Ok(data)
}

fn generate_paths(params: &SpectreParams) -> SpectreGeneratedPaths {
    SpectreGeneratedPaths {
        raw_output_dir: params.work_dir.join("psf/"),
        log_path: params.work_dir.join("spectre.log"),
        stdout_path: params.work_dir.join("spectre.out"),
        stderr_path: params.work_dir.join("spectre.err"),
    }
}

pub fn run_sram_spectre(params: &SramParams, work_dir: impl AsRef<Path>, name: &str) -> Result<()> {
    let &SramParams {
        wmask_width,
        data_width,
        addr_width,
        ..
    } = params;
    let alternating_bits = 0b0101010101010101010101010101010101010101010101010101010101010101u64;
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
    if wmask_width > 1 {
        ports.push((PortClass::WriteMask, PortOrder::MsbFirst));
    }
    let mut tb = TbParams::builder();
    tb.test_case(test_case)
        .sram_name(name)
        .tr(50e-12)
        .tf(50e-12)
        .vdd(1.8)
        .c_load(5e-15)
        .data_width(data_width)
        .addr_width(addr_width)
        .wmask_width(wmask_width)
        .ports(ports)
        .clk_port("clk")
        .write_enable_port("we")
        .addr_port("addr")
        .data_in_port("din")
        .data_out_port("dout")
        .pwr_port("vdd")
        .gnd_port("vss")
        .wmask_port("wmask")
        .work_dir(std::path::PathBuf::from(work_dir.as_ref()).join("sim"))
        .source_paths(source_files(&work_dir, name, VerificationTask::SpectreSim));

    tb.includes(crate::verification::spectre::sky130_includes());

    let tb = tb.build()?;

    verification::run_testbench(&tb).with_context(|| "Error simulating testbench")?;

    Ok(())
}
