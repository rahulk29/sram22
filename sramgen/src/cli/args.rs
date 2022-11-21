use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
pub struct Args {
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,
    #[cfg(feature = "calibre")]
    #[arg(short, long)]
    pub drc: bool,
    #[cfg(feature = "spectre")]
    #[arg(short, long)]
    pub simulate: bool,
    #[cfg(any(feature = "calibre", feature = "spectre"))]
    #[arg(short, long)]
    pub all_tests: bool,
}
