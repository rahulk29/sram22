use clap::Parser;
use std::path::PathBuf;

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
    #[arg(short, long, default_value = "sramgen.toml")]
    pub config: PathBuf,

    /// Directory to which output files should be saved.
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Generate LEF (used in place and route).
    #[cfg(feature = "abstract_lef")]
    #[arg(long)]
    pub lef: bool,

    /// Generate LIB (setup, hold, and delay timing information).
    #[cfg(feature = "liberate_mx")]
    #[arg(long)]
    pub lib: bool,

    /// Run DRC using Calibre.
    #[cfg(feature = "calibre")]
    #[arg(long)]
    pub drc: bool,

    /// Run LVS using Calibre.
    #[cfg(feature = "calibre")]
    #[arg(long)]
    pub lvs: bool,

    /// Run PEX using Calibre.
    #[cfg(all(feature = "calibre", feature = "pex"))]
    #[arg(long)]
    pub pex: bool,

    /// Run Spectre to verify SRAM functionality.
    #[cfg(feature = "spectre")]
    #[arg(long)]
    pub sim: bool,

    /// Run all available steps.
    #[cfg(any(
        feature = "abstract_lef",
        feature = "liberate_mx",
        feature = "calibre",
        feature = "spectre"
    ))]
    #[arg(short, long)]
    pub all: bool,
}
