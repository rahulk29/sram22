pub mod args;

use crate::cli::args::Args;
use anyhow::Result;
use clap::Parser;

pub fn run() -> Result<()> {
    let args = Args::parse();
    println!("{:?}", args.config);
    Ok(())
}
