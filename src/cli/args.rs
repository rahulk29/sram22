use clap::Parser;
use std::path::PathBuf;

// TODO: Add option to run Spectre simulations.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about,
    help_template(
        "{before-help}{name} {version}\n{author-with-newline}{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}{after-help}"
    )
)]
pub struct Args {
    /// Path to TOML configuration file.
    #[arg(short, long, default_value = "sram22.toml")]
    pub config: PathBuf,

    /// Directory to which output files should be saved.
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Generate LEF (used in place and route).
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub lef: bool,

    /// Generate LIB (setup, hold, and delay timing information).
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub lib: bool,

    /// Run DRC using Calibre.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub drc: bool,

    /// Run LVS using Calibre.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub lvs: bool,

    /// Run PEX using Calibre.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub pex: bool,

    /// Run all available steps.
    #[cfg(feature = "commercial")]
    #[arg(short, long)]
    pub all: bool,
}
