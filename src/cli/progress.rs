use std::collections::HashSet;
use std::fmt::Display;
use std::time::Duration;

use anyhow::Error;
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::plan::TaskKey;
use crate::Result;

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
    key: TaskKey,
    progress_bar: ProgressBar,
    disabled: bool,
}

impl Step {
    fn matches(&self, key: TaskKey) -> bool {
        self.key == key
    }
}

impl StepContext {
    #[allow(unused_variables)]
    pub fn new_with_mp(tasks: &HashSet<TaskKey>, mp: MultiProgress, prefix: &str) -> Self {
        let mut steps = vec![
            Step {
                desc: "Generate plan".to_string(),
                key: TaskKey::GeneratePlan,
                progress_bar: ProgressBar::new_spinner(),
                disabled: false,
            },
            Step {
                desc: "Generate netlist".to_string(),
                key: TaskKey::GenerateNetlist,
                progress_bar: ProgressBar::new_spinner(),
                disabled: false,
            },
            Step {
                desc: "Generate layout".to_string(),
                key: TaskKey::GenerateLayout,
                progress_bar: ProgressBar::new_spinner(),
                disabled: false,
            },
            Step {
                desc: "Generate Verilog".to_string(),
                key: TaskKey::GenerateVerilog,
                progress_bar: ProgressBar::new_spinner(),
                disabled: false,
            },
            Step {
                desc: "Generate LEF".to_string(),
                key: TaskKey::GenerateLef,
                progress_bar: ProgressBar::new_spinner(),
                disabled: false,
            },
            #[cfg(feature = "commercial")]
            Step {
                desc: "Run DRC".to_string(),
                key: TaskKey::RunDrc,
                progress_bar: ProgressBar::new_spinner(),
                disabled: !tasks.contains(&TaskKey::RunDrc) && !tasks.contains(&TaskKey::All),
            },
            #[cfg(feature = "commercial")]
            Step {
                desc: "Run LVS".to_string(),
                key: TaskKey::RunLvs,
                progress_bar: ProgressBar::new_spinner(),
                disabled: !tasks.contains(&TaskKey::RunLvs) && !tasks.contains(&TaskKey::All),
            },
            #[cfg(feature = "commercial")]
            Step {
                desc: "Run PEX".to_string(),
                key: TaskKey::RunPex,
                progress_bar: ProgressBar::new_spinner(),
                disabled: !tasks.contains(&TaskKey::RunPex),
            },
            Step {
                desc: "Generate LIB".to_string(),
                key: TaskKey::GenerateLib,
                progress_bar: ProgressBar::new_spinner(),
                disabled: false,
            },
        ];

        let num_steps = steps.iter().filter(|step| !step.disabled).count();
        let mut counter = 0;
        let width = format!("{num_steps}").len();
        let prefix_str = if prefix.is_empty() {
            String::new()
        } else {
            format!("[{}] ", prefix)
        };

        for step in steps.iter_mut() {
            mp.add(step.progress_bar.clone());
            if step.disabled {
                let msg = Some(format!("{}[-/-] {}", prefix_str, step.desc));
                step.set_status(StepStatus::Disabled, msg);
            } else {
                counter += 1;
                let msg = Some(format!(
                    "{}[{:width$}/{:width$}] {}",
                    prefix_str, counter, num_steps, step.desc
                ));
                step.set_status(StepStatus::Pending, msg);
            }
        }
        if !steps.is_empty() {
            steps[0]
                .progress_bar
                .enable_steady_tick(Duration::from_millis(200));
        }
        StepContext { step_num: 0, steps }
    }

    pub fn advance(&mut self) {
        self.step_num += 1;
        while let Some(current_step) = self.current_step() {
            if !current_step.disabled {
                break;
            }
            self.step_num += 1;
        }
    }

    #[inline]
    pub fn current_step(&mut self) -> Option<&mut Step> {
        if self.step_num < self.steps.len() {
            Some(&mut self.steps[self.step_num])
        } else {
            None
        }
    }

    pub fn check<T>(&mut self, res: Result<T>) -> Result<T> {
        if res.is_err() {
            if let Some(current_step) = self.current_step() {
                current_step.set_status(StepStatus::Failed, None);
                self.advance();
                while let Some(current_step) = self.current_step() {
                    current_step.set_status(StepStatus::Skipped, None);
                    self.advance();
                }
            }
        }
        res
    }

    pub fn bail(&mut self, e: Error) -> Result<()> {
        self.check(Err(e))
    }

    pub fn finish(&mut self, key: TaskKey) {
        if let Some(current_step) = self.current_step() {
            if !current_step.matches(key) {
                panic!("A step was completed out of order");
            }
            current_step.set_status(StepStatus::Done, None);

            self.advance();

            if let Some(current_step) = self.current_step() {
                current_step.set_status(StepStatus::InProgress, None);
            }
        } else if !self.steps.last().is_some_and(|s| s.matches(key)) {
            panic!("A step was completed after all steps were marked completed");
        }
    }

    pub fn commit(&mut self) {
        for step in &self.steps {
            step.progress_bar.finish();
        }
    }
}

fn format_template(spinner: bool, status: impl Display) -> String {
    if spinner {
        format!("{{spinner:.green}} {status:16} {{msg}}")
    } else {
        format!("  {status:16} {{msg}}")
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
            self.progress_bar.disable_steady_tick();
            self.progress_bar.tick();
        }
    }
}
