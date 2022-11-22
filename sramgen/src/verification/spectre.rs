use anyhow::bail;
use psf_ascii::parser::transient::TransientData;
use std::path::PathBuf;
use std::process::Command;

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
