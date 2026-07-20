use std::collections::HashSet;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::sync::Arc;

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
    // Completeness is judged on the layout artifacts only, not the LIB. A LIB is
    // interpolated by default when possible, but not every SRAM config has timing
    // data to interpolate from, so a missing .lib does not mean the build is stale.
    out_spice(work_dir, name).exists()
        && out_gds(work_dir, name).exists()
        && out_verilog(work_dir, name).exists()
        && out_lef(work_dir, name).exists()
}

/// Layer per-config tasks onto the shared base set. A config runs PEX exactly
/// when it specifies a `pex_level`.
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

    // Every run generates a LIB; DRC, LVS, and the `all` umbrella are opt-in.
    // PEX is decided per config in `config_tasks` from each config's pex_level.
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

    let mp = MultiProgress::new();

    let handles: Vec<_> = plans
        .into_iter()
        .zip(configs.into_iter())
        .filter_map(|(plan, config)| {
            let work_dir_check = build_dir.join(plan.sram_params.name().as_str());
            if is_already_built(&work_dir_check, &plan.sram_params.name()) {
                return None;
            }

            let tasks = config_tasks(&base_tasks, &config);

            let mut ctx = StepContext::new_with_mp(&tasks, mp.clone(), &plan.sram_params.name());
            ctx.finish(TaskKey::GeneratePlan);

            let build_dir = build_dir.clone();
            Some(std::thread::spawn(move || -> (Result<PathBuf>, StepContext) {
                let result = (|| -> Result<PathBuf> {
                    let work_dir = build_dir.join(plan.sram_params.name().as_str());
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
                        use_liberate: args.liberate || args.all,
                    });
                    ctx.check(res)?;
                    Ok(work_dir)
                })();
                (result, ctx)
            }))
        })
        .collect();

    // Join ALL threads first, then commit ALL progress bars together to avoid
    // ghost snapshots that appear when one bar finishes while others are live.
    let joined: Vec<_> = handles.into_iter().map(|h| h.join()).collect();
    let mut errors: Vec<anyhow::Error> = Vec::new();
    let mut work_dirs: Vec<PathBuf> = Vec::new();
    for join_result in joined {
        match join_result {
            Ok((Ok(work_dir), mut ctx)) => {
                ctx.commit();
                work_dirs.push(work_dir);
            }
            Ok((Err(e), mut ctx)) => {
                ctx.commit();
                errors.push(e);
            }
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| e.downcast_ref::<&'static str>().copied())
                    .unwrap_or("(no message)");
                errors.push(anyhow::anyhow!("SRAM generation thread panicked: {}", msg));
            }
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
