use std::collections::HashSet;
use std::fs::canonicalize;
use std::path::PathBuf;

use clap::Parser;

use crate::cli::args::Args;
use crate::cli::progress::StepContext;
use crate::config::sram::parse_sram_config;
use crate::paths::out_sram;
use crate::plan::extract::ExtractionResult;
use crate::plan::{execute_plan, generate_plan, ExecutePlanParams, TaskKey};
use crate::Result;

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

    let config_path = canonicalize(&args.config)?;

    println!("{}", BANNER);
    println!("Starting SRAM generation...\n");

    let config = parse_sram_config(&config_path)?;

    let name = &out_sram(&config);

    let work_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from(name)
    };
    std::fs::create_dir_all(&work_dir)?;
    let work_dir = canonicalize(work_dir)?;

    println!("Configuration file: {:?}", &config_path);
    println!("Output directory: {:?}\n", &work_dir);
    println!("SRAM parameters:");
    println!("\tNumber of words: {}", config.num_words);
    println!("\tData width: {}", config.data_width);
    println!("\tMux ratio: {}", config.mux_ratio);
    println!("\tWrite size: {}", config.write_size);
    println!("\tControl mode: {:?}\n", config.control);

    let enabled_tasks = vec![
        #[cfg(feature = "commercial")]
        (args.lef, TaskKey::GenerateLef),
        #[cfg(feature = "commercial")]
        (args.drc, TaskKey::RunDrc),
        #[cfg(feature = "commercial")]
        (args.lvs, TaskKey::RunLvs),
        #[cfg(feature = "commercial")]
        (args.pex, TaskKey::RunPex),
        #[cfg(feature = "commercial")]
        (args.lib, TaskKey::GenerateLib),
        #[cfg(feature = "commercial")]
        (args.sim, TaskKey::RunSpectre),
        #[cfg(feature = "commercial")]
        (args.all, TaskKey::All),
    ]
    .into_iter()
    .filter_map(|(a, b)| if a { Some(b) } else { None });

    let tasks = HashSet::from_iter(enabled_tasks);

    let mut ctx = StepContext::new(&tasks);

    let plan = ctx.check(generate_plan(ExtractionResult {}, &config))?;
    ctx.finish(TaskKey::GeneratePlan);

    let res = execute_plan(ExecutePlanParams {
        work_dir: &work_dir,
        plan: &plan,
        tasks: &tasks,
        ctx: Some(&mut ctx),
        pex_level: config.pex_level,
    });

    ctx.check(res)?;

    Ok(())
}
