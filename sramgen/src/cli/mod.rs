use std::fs::canonicalize;
use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;

use crate::cli::args::Args;
use crate::cli::progress::{StepContext, StepKey};
use crate::config::sram::parse_sram_config;
use crate::paths::out_sram;
use crate::plan::extract::ExtractionResult;
use crate::plan::{execute_plan, generate_plan};
use crate::{Result, BUILD_PATH};

pub mod args;
pub mod progress;

pub const BANNER: &str = r"
 ________  ________  ________  _____ ______     _______   _______     
|\   ____\|\   __  \|\   __  \|\   _ \  _   \  /  ___  \ /  ___  \    
\ \  \___|\ \  \|\  \ \  \|\  \ \  \\\__\ \  \/__/|_/  //__/|_/  /|   
 \ \_____  \ \   _  _\ \   __  \ \  \\|__| \  \__|//  / /__|//  / /   
  \|____|\  \ \  \\  \\ \  \ \  \ \  \    \ \  \  /  /_/__  /  /_/__  
    ____\_\  \ \__\\ _\\ \__\ \__\ \__\    \ \__\|\________\\________\
   |\_________\|__|\|__|\|__|\|__|\|__|     \|__| \|_______|\|_______|
   \|_________|                                                       
                                                                      
                                                                      
";

pub fn run() -> Result<()> {
    let args = Args::parse();

    let config_path = canonicalize(args.config)?;

    println!("{}", BANNER);
    println!("Starting SRAM generation...\n");

    let config = parse_sram_config(&config_path)?;

    let name = &out_sram(&config);

    let work_dir = canonicalize(if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from(name)
    })?;

    println!("Configuration file: {:?}", &config_path);
    println!("Output directory: {:?}\n", &work_dir);
    println!("SRAM parameters:");
    println!("\tNumber of words: {}", config.num_words);
    println!("\tData width: {}", config.data_width);
    println!("\tMux ratio: {}", config.mux_ratio);
    println!("\tWrite size: {}", config.write_size);
    println!("\tControl mode: {:?}\n", config.control);

    let mut ctx = StepContext::new(args.quick);

    let plan = ctx.check(generate_plan(ExtractionResult {}, &config))?;
    ctx.finish(StepKey::GeneratePlan);

    let res = execute_plan(&work_dir, &plan, args.quick, Some(&mut ctx));
    ctx.check(res)?;

    Ok(())
}
