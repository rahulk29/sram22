use std::fmt::Display;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Error};
use clap::Parser;
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::cli::args::Args;
use crate::config::sram::parse_sram_config;
use crate::plan::extract::ExtractionResult;
use crate::plan::{execute_plan, generate_plan};
use crate::{Result, BUILD_PATH};

pub mod args;

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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StepKey {
    ParseConfig,
    GeneratePlan,
    GenerateNetlist,
    GenerateLayout,
    GenerateVerilog,
    #[cfg(feature = "abstract_lef")]
    GenerateLef,
    #[cfg(feature = "calibre")]
    RunDrc,
    #[cfg(feature = "calibre")]
    RunLvs,
    #[cfg(all(feature = "calibre", feature = "pex"))]
    RunPex,
    #[cfg(feature = "spectre")]
    RunSpectre,
}

#[derive(PartialEq, Eq)]
pub enum StepStatus {
    Done,
    Pending,
    InProgress,
    Disabled,
    Skipped,
    Failed,
}

pub struct StepContext {
    step_num: usize,
    steps: Vec<Step>,
}

pub struct Step {
    desc: String,
    key: StepKey,
    progress_bar: ProgressBar,
    disabled: bool,
}

impl StepContext {
    pub fn new(steps: Vec<Step>) -> Self {
        if !steps.is_empty() {
            steps[0]
                .progress_bar
                .enable_steady_tick(Duration::from_millis(200));
        }
        StepContext { step_num: 0, steps }
    }

    #[inline]
    pub fn current_step(&mut self) -> &mut Step {
        &mut self.steps[self.step_num]
    }

    pub fn check<T>(&mut self, res: Result<T>) -> Result<T> {
        if res.is_err() {
            if self.step_num < self.steps.len() {
                let current_step = self.current_step();
                current_step.set_status(StepStatus::Failed, None);
                while self.step_num < self.steps.len() - 1 {
                    self.step_num += 1;
                    let current_step = self.current_step();
                    if !current_step.disabled {
                        current_step.set_status(StepStatus::Skipped, None);
                    }
                }
            }
            println!("\n");
        }

        res
    }

    pub fn bail(&mut self, e: Error) -> Result<()> {
        self.check(Err(e))
    }

    pub fn finish(&mut self, key: StepKey) {
        if self.step_num >= self.steps.len() {
            panic!("A step was completed after all steps were marked completed");
        }

        let current_step = self.current_step();

        if current_step.key != key {
            panic!("A step was completed out of order");
        }

        current_step.set_status(StepStatus::Done, None);

        self.step_num += 1;

        if self.step_num == self.steps.len() {
            self.done();
        } else {
            self.current_step().set_status(StepStatus::InProgress, None);
        }
    }

    pub fn done(&mut self) {
        println!("\n\nCompleted all tasks");
    }
}

fn format_template(spinner: bool, status: impl Display) -> String {
    if spinner {
        format!("{{spinner:.green}} {:16} {{msg}}", status)
    } else {
        format!("  {:16} {{msg}}", status)
    }
}

impl Step {
    fn set_status(&mut self, status: StepStatus, msg: Option<String>) {
        let status_template = match status {
            StepStatus::Disabled => {
                format_template(false, "Disabled".truecolor(120, 120, 120).bold())
            }
            StepStatus::Done => format_template(false, "Done".green().bold()),
            StepStatus::Failed => format_template(false, "Failed".bright_white().on_red().bold()),
            StepStatus::InProgress => format_template(true, "In Progress".bright_white().bold()),
            StepStatus::Pending => format_template(true, "Pending".blue().bold()),
            StepStatus::Skipped => format_template(false, "Skipped".yellow().bold()),
        };
        self.progress_bar
            .set_style(ProgressStyle::with_template(&status_template).unwrap());

        if let Some(msg) = msg {
            self.progress_bar.set_message(msg);
        }

        if status == StepStatus::InProgress {
            self.progress_bar
                .enable_steady_tick(Duration::from_millis(200));
        } else if status != StepStatus::Pending {
            self.progress_bar.finish();
        }
    }
}

pub fn run() -> Result<()> {
    let args = Args::parse();

    println!("{}", BANNER);

    println!("Starting SRAM generation...\n");

    println!("Tasks:");

    let mp = MultiProgress::new();

    let mut steps = vec![
        Step {
            desc: "Parse configuration file".to_string(),
            key: StepKey::ParseConfig,
            progress_bar: ProgressBar::new_spinner(),
            disabled: false,
        },
        Step {
            desc: "Generate plan".to_string(),
            key: StepKey::GeneratePlan,
            progress_bar: ProgressBar::new_spinner(),
            disabled: false,
        },
        Step {
            desc: "Generate netlist".to_string(),
            key: StepKey::GenerateNetlist,
            progress_bar: ProgressBar::new_spinner(),
            disabled: false,
        },
        Step {
            desc: "Generate layout".to_string(),
            key: StepKey::GenerateLayout,
            progress_bar: ProgressBar::new_spinner(),
            disabled: false,
        },
        Step {
            desc: "Generate Verilog".to_string(),
            key: StepKey::GenerateVerilog,
            progress_bar: ProgressBar::new_spinner(),
            disabled: false,
        },
        #[cfg(feature = "abstract_lef")]
        Step {
            desc: "Generate LEF".to_string(),
            key: StepKey::GenerateLef,
            progress_bar: ProgressBar::new_spinner(),
            disabled: false,
        },
        #[cfg(feature = "calibre")]
        Step {
            desc: "Run DRC".to_string(),
            key: StepKey::RunDrc,
            progress_bar: ProgressBar::new_spinner(),
            disabled: !args.drc && !args.all_tests,
        },
        #[cfg(feature = "calibre")]
        Step {
            desc: "Run LVS".to_string(),
            key: StepKey::RunLvs,
            progress_bar: ProgressBar::new_spinner(),
            disabled: !args.lvs && !args.all_tests,
        },
        #[cfg(all(feature = "calibre", feature = "pex"))]
        Step {
            desc: "Run PEX".to_string(),
            key: StepKey::RunPex,
            progress_bar: ProgressBar::new_spinner(),
            disabled: !args.pex && !args.all_tests,
        },
        #[cfg(feature = "spectre")]
        Step {
            desc: "Run Spectre".to_string(),
            key: StepKey::RunPex,
            progress_bar: ProgressBar::new_spinner(),
            disabled: !args.spectre && !args.all_tests,
        },
    ];

    let num_steps = steps.iter().filter(|step| !step.disabled).count();
    let mut counter = 0;
    let width = format!("{}", num_steps).len();
    for (i, step) in steps.iter_mut().enumerate() {
        mp.insert(i + 1, step.progress_bar.clone());
        if step.disabled {
            let msg = Some(format!("[-/-] {}", step.desc));
            step.set_status(StepStatus::Disabled, msg);
        } else {
            counter += 1;
            let msg = Some(format!(
                "[{:width$}/{:width$}] {}",
                counter, num_steps, step.desc
            ));
            step.set_status(StepStatus::Pending, msg);
        }
    }

    let mut ctx = StepContext::new(steps);

    let config_path = if let Some(config) = args.config {
        config
    } else if std::fs::metadata("./sramgen.toml").is_ok() {
        PathBuf::from("./sramgen.toml")
    } else {
        return ctx.bail(anyhow!(
            "Could not find `sramgen.toml` in the current working directory."
        ));
    };
    let config = ctx.check(parse_sram_config(config_path))?;
    ctx.finish(StepKey::ParseConfig);

    let plan = ctx.check(generate_plan(ExtractionResult {}, &config))?;
    ctx.finish(StepKey::GeneratePlan);

    let name = &plan.sram_params.name;
    let work_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from(BUILD_PATH).join(name)
    };
    let res = execute_plan(&work_dir, &plan, Some(&mut ctx));
    ctx.check(res)?;

    #[cfg(feature = "calibre")]
    {
        if args.drc || args.all_tests {
            crate::verification::calibre::run_sram_drc(&work_dir, name)?;
        }
        if args.lvs || args.all_tests {
            crate::verification::calibre::run_sram_lvs(&work_dir, name)?;
        }
        #[cfg(feature = "pex")]
        if args.pex || args.all_tests {
            crate::verification::calibre::run_sram_pex(&work_dir, name)?;
        }
    }

    #[cfg(feature = "spectre")]
    if args.spectre || args.all_tests {
        crate::verification::spectre::run_sram_spectre(&plan.sram_params, &work_dir, name)?;
    }

    Ok(())
}
