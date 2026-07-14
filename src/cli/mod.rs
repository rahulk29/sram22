use std::collections::HashSet;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;

use indicatif::MultiProgress;

use crate::blocks::sram::parse_sram_batch_config;
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

fn wants_lib(tasks: &HashSet<TaskKey>) -> bool {
    if tasks.contains(&TaskKey::GenerateLib) {
        return true;
    }
    #[cfg(feature = "commercial")]
    if tasks.contains(&TaskKey::All) {
        return true;
    }
    false
}

fn is_already_built(work_dir: &std::path::Path, name: &str, check_lib: bool) -> bool {
    let layout_done = out_spice(work_dir, name).exists()
        && out_gds(work_dir, name).exists()
        && out_verilog(work_dir, name).exists()
        && out_lef(work_dir, name).exists();

    if !layout_done {
        return false;
    }

    if check_lib {
        let lib_suffixes = ["tt_025C_1v80", "ss_100C_1v60", "ff_n40C_1v95"];
        let libs_done = lib_suffixes.iter().all(|suffix| {
            work_dir.join(format!("{}_{}.lib", name, suffix)).exists()
        });
        if !libs_done {
            return false;
        }
    }

    true
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

    let enabled_tasks = vec![
        #[cfg(feature = "commercial")]
        (args.drc, TaskKey::RunDrc),
        #[cfg(feature = "commercial")]
        (args.lvs, TaskKey::RunLvs),
        #[cfg(feature = "commercial")]
        (
            args.pex || (args.lib && configs.iter().any(|c| c.pex_level.is_some())),
            TaskKey::RunPex,
        ),
        (args.lib, TaskKey::GenerateLib),
        #[cfg(feature = "commercial")]
        (args.all, TaskKey::All),
    ]
    .into_iter()
    .filter_map(|(a, b)| if a { Some(b) } else { None });

    let tasks = Arc::new(HashSet::from_iter(enabled_tasks));

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

    // SRAMs run concurrently: plan/netlist/layout/verilog/LEF generation
    // and the open-source interpolated LIB model have no shared resource
    // constraints. PEX extraction and Liberate MX characterization are
    // throttled separately (see `plan::HEAVY_BACKEND_SLOT`) so they don't
    // exhaust the license pool or crash the server on memory/CPU, without
    // limiting everything else to running one SRAM at a time.
    let handles: Vec<_> = plans
        .into_iter()
        .zip(configs.into_iter())
        .filter_map(|(plan, config)| {
            let work_dir_check = build_dir.join(plan.sram_params.name().as_str());
            if is_already_built(&work_dir_check, &plan.sram_params.name(), wants_lib(&tasks)) {
                return None;
            }

            let mut ctx = StepContext::new_with_mp(&tasks, mp.clone(), &plan.sram_params.name());
            ctx.finish(TaskKey::GeneratePlan);

            let tasks = Arc::clone(&tasks);
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
            Err(_) => errors.push(anyhow::anyhow!("An SRAM generation thread panicked")),
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
