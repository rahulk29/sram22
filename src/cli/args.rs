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

    /// Generate LIB timing using Liberate MX SPICE characterization instead of
    /// the open-source interpolation model (requires a full installation).
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub liberate: bool,

    /// Run DRC using Calibre.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub drc: bool,

    /// Run LVS using Calibre.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub lvs: bool,

    #[cfg(feature = "commercial")]
    /// Run all available steps.
    #[arg(short, long)]
    pub all: bool,

    /// Maximum number of SRAMs to generate concurrently. Each runs its own PEX
    /// and Liberate MX steps, which use licensed tools, so raise this only if
    /// your machine has the licenses and compute to spare.
    #[cfg(feature = "commercial")]
    #[arg(short = 'p', long, default_value_t = 1)]
    pub parallel: usize,
}
