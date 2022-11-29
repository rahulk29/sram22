pub mod args;

use crate::cli::args::Args;
use crate::config::sram::parse_sram_config;
use crate::plan::extract::ExtractionResult;
use crate::plan::{execute_plan, generate_plan};
use crate::BUILD_PATH;
use anyhow::{bail, Result};
use clap::Parser;
use std::path::PathBuf;

pub fn run() -> Result<()> {
    let args = Args::parse();
    let config_path = if let Some(config) = args.config {
        config
    } else if std::fs::metadata("./sramgen.toml").is_ok() {
        PathBuf::from("./sramgen.toml")
    } else {
        bail!("Could not find `sramgen.toml` in the current working directory.");
    };
    let config = parse_sram_config(config_path)?;
    let plan = generate_plan(ExtractionResult {}, &config)?;
    let name = &plan.sram_params.name;
    let work_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from(BUILD_PATH).join(name)
    };
    execute_plan(&work_dir, &plan)?;

    #[cfg(feature = "calibre")]
    {
        if args.drc || args.all_tests {
            crate::verification::calibre::run_sram_drc(&work_dir, name)?;
        }
        if args.lvs || args.all_tests {
            crate::verification::calibre::run_sram_lvs(&work_dir, name, config.control)?;
        }
        #[cfg(feature = "pex")]
        if args.pex || args.all_tests {
            crate::verification::calibre::run_sram_pex(&work_dir, name, config.control)?;
        }
    }

    #[cfg(feature = "spectre")]
    if args.spectre || args.all_tests {
        crate::verification::spectre::run_sram_spectre(&plan.sram_params, &work_dir, name)?;
    }

    Ok(())
}
