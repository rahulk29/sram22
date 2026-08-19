use std::collections::HashSet;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clap::Parser;

use indicatif::MultiProgress;

use crate::blocks::sram::{parse_sram_batch_config, SramConfig};
use crate::cli::args::Args;
use crate::cli::progress::StepContext;
use crate::paths::{out_gds, out_lef, out_spice, out_verilog};
use crate::plan::{execute_plan, generate_plan, ExecutePlanParams, SramPlan, TaskKey};
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

fn is_already_built(work_dir: &std::path::Path, name: &str) -> bool {
    out_spice(work_dir, name).exists()
        && out_gds(work_dir, name).exists()
        && out_verilog(work_dir, name).exists()
        && out_lef(work_dir, name).exists()
}

#[cfg(feature = "commercial")]
fn config_tasks(base: &HashSet<TaskKey>, config: &SramConfig) -> Arc<HashSet<TaskKey>> {
    let mut tasks = base.clone();
    if config.pex_level.is_some() {
        tasks.insert(TaskKey::RunPex);
    }
    Arc::new(tasks)
}

#[cfg(not(feature = "commercial"))]
fn config_tasks(base: &HashSet<TaskKey>, _config: &SramConfig) -> Arc<HashSet<TaskKey>> {
    Arc::new(base.clone())
}

pub fn run() -> Result<()> {
    let args = Args::parse();

    let config_path = canonicalize(&args.config)?;

    println!("{BANNER}");

    println!("Reading configuration file...\n");
    let configs = parse_sram_batch_config(&config_path)?;

    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Config path has no parent directory"))?;

    let build_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        config_dir.join("build")
    };
    std::fs::create_dir_all(&build_dir)?;
    let build_dir = canonicalize(build_dir)?;

    let base_tasks: HashSet<TaskKey> = [
        (true, TaskKey::GenerateLib),
        #[cfg(feature = "commercial")]
        (args.drc, TaskKey::RunDrc),
        #[cfg(feature = "commercial")]
        (args.lvs, TaskKey::RunLvs),
        #[cfg(feature = "commercial")]
        (args.all, TaskKey::All),
    ]
    .into_iter()
    .filter_map(|(enabled, task)| enabled.then_some(task))
    .collect();

    let plans: Vec<SramPlan> = configs
        .iter()
        .map(|c| generate_plan(c))
        .collect::<Result<Vec<_>>>()?;

    println!("Configuration file: {:?}", &config_path);
    for (i, (config, plan)) in configs.iter().zip(plans.iter()).enumerate() {
        println!(
            "  [{}] {} (num_words={}, data_width={}, mux_ratio={}, write_size={})",
            i + 1,
            plan.sram_params.name(),
            config.num_words,
            config.data_width,
            config.mux_ratio as usize,
            config.write_size,
        );
    }
    println!();

    let work_items: Vec<(SramPlan, SramConfig)> = plans
        .into_iter()
        .zip(configs)
        .filter(|(plan, _)| {
            let work_dir = build_dir.join(plan.sram_params.name().as_str());
            !is_already_built(&work_dir, &plan.sram_params.name())
        })
        .collect();

    let num_workers = match args.parallel {
        Some(limit) => work_items.len().min(limit.max(1)),
        None => work_items.len(),
    };

    #[cfg(feature = "commercial")]
    let use_liberate = args.liberate;

    let mp = MultiProgress::new();
    let queue = Arc::new(Mutex::new(work_items.into_iter()));
    let results: Arc<Mutex<Vec<Result<PathBuf>>>> = Arc::new(Mutex::new(Vec::new()));

    let workers: Vec<_> = (0..num_workers)
        .map(|_| {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let base_tasks = base_tasks.clone();
            let build_dir = build_dir.clone();
            let mp = mp.clone();
            std::thread::spawn(move || loop {
                let item = queue.lock().unwrap().next();
                let Some((plan, config)) = item else { break };
                let name = plan.sram_params.name();
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let tasks = config_tasks(&base_tasks, &config);
                    let mut ctx = StepContext::new_with_mp(&tasks, mp.clone(), &name);
                    ctx.finish(TaskKey::GeneratePlan);
                    let result = (|| -> Result<PathBuf> {
                        let work_dir = build_dir.join(name.as_str());
                        std::fs::create_dir_all(&work_dir)?;
                        let work_dir = canonicalize(work_dir)?;
                        let res = execute_plan(ExecutePlanParams {
                            work_dir: &work_dir,
                            plan: &plan,
                            tasks,
                            ctx: Some(&mut ctx),
                            #[cfg(feature = "commercial")]
                            pex_level: config.pex_level,
                            #[cfg(feature = "commercial")]
                            use_liberate,
                        });
                        ctx.check(res)?;
                        Ok(work_dir)
                    })();
                    ctx.commit();
                    result
                }));
                let result = outcome.unwrap_or_else(|panic| {
                    let msg = panic
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic.downcast_ref::<&'static str>().copied())
                        .unwrap_or("(no message)");
                    Err(anyhow::anyhow!(
                        "SRAM generation panicked for {}: {}",
                        name,
                        msg
                    ))
                });
                results.lock().unwrap().push(result);
            })
        })
        .collect();

    for worker in workers {
        let _ = worker.join();
    }

    let mut errors: Vec<anyhow::Error> = Vec::new();
    let mut work_dirs: Vec<PathBuf> = Vec::new();
    for result in Arc::try_unwrap(results).unwrap().into_inner().unwrap() {
        match result {
            Ok(work_dir) => work_dirs.push(work_dir),
            Err(e) => errors.push(e),
        }
    }
    for work_dir in work_dirs {
        println!("Artifacts saved to: {:?}", work_dir);
    }

    if !errors.is_empty() {
        let msg = errors
            .iter()
            .enumerate()
            .map(|(i, e)| format!("  [{}] {:#}", i + 1, e))
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!("{} SRAM(s) failed:\n{}", errors.len(), msg);
    }

    Ok(())
}
