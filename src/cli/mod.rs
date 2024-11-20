use std::collections::HashSet;
use std::fs::canonicalize;
use std::path::PathBuf;

use clap::Parser;

use crate::blocks::sram::parse_sram_config;
use crate::cli::args::Args;
use crate::cli::progress::StepContext;
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
                                                                      
                                                                      
SRAM22 v0.2
";

pub fn run() -> Result<()> {
    let args = Args::parse();

    let config_path = canonicalize(&args.config)?;

    println!("{BANNER}");

    println!("Reading configuration file...\n");
    let config = parse_sram_config(&config_path)?;

    println!("Configuration file: {:?}", &config_path);
    println!("SRAM parameters:");
    println!("\tNumber of words: {}", config.num_words);
    println!("\tData width: {}", config.data_width);
    println!("\tMux ratio: {}", config.mux_ratio as usize);
    println!("\tWrite size: {}", config.write_size);

    let enabled_tasks = vec![
        #[cfg(feature = "commercial")]
        (args.lef, TaskKey::GenerateLef),
        #[cfg(feature = "commercial")]
        (args.drc, TaskKey::RunDrc),
        #[cfg(feature = "commercial")]
        (args.lvs, TaskKey::RunLvs),
        #[cfg(feature = "commercial")]
        (
            args.pex || (args.lib && config.pex_level.is_some()),
            TaskKey::RunPex,
        ),
        #[cfg(feature = "commercial")]
        (args.lib, TaskKey::GenerateLib),
        #[cfg(feature = "commercial")]
        (args.all, TaskKey::All),
    ]
    .into_iter()
    .filter_map(|(a, b)| if a { Some(b) } else { None });

    let tasks = HashSet::from_iter(enabled_tasks);

    let mut ctx = StepContext::new(&tasks);

    let plan = ctx.check(generate_plan(&config))?;
    ctx.finish(TaskKey::GeneratePlan);

    let work_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from(plan.sram_params.name().as_str())
    };
    std::fs::create_dir_all(&work_dir)?;
    let work_dir = canonicalize(work_dir)?;

    let res = execute_plan(ExecutePlanParams {
        work_dir: &work_dir,
        plan: &plan,
        tasks: &tasks,
        ctx: Some(&mut ctx),
        #[cfg(feature = "commercial")]
        pex_level: config.pex_level,
    });

    ctx.check(res)?;
    println!("Artifacts saved to: {:?}\n", &work_dir);

    Ok(())
}
