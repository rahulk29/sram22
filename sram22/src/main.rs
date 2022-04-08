use std::path::Path;

use sram22::{config::SramConfig, generate};

use clap::Parser;

// fn main() {
//     env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
//
//     let config = SramConfig {
//         rows: 16,
//         cols: 16,
//         output_dir: "/home/rahul/acads/sky130/sram22/_build".to_string(),
//         cell_dir: "/home/rahul/acads/sky130/sram22/tech/sky130/magic".to_string(),
//     };
//
//     generate(config).expect("failed to generate SRAM");
// }

#[derive(Parser)]
#[clap(author, version, about = "A configurable SRAM generator", long_about = None)]
struct Cli {
    /// Path to a TOML configuration file specifying memory options
    config: String,

    /// Generate SPICE netlists only. No layouts will be generated.
    #[clap(short, long)]
    netlist_only: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    let cli = Cli::parse();

    let cfg_path = Path::new(&cli.config);
    let cfg_path = if cfg_path.is_relative() {
        let mut cwd = std::env::current_dir()?;
        cwd.push(cfg_path);
        cwd
    } else {
        cfg_path.to_owned()
    };

    let s = std::fs::read_to_string(&cfg_path)?;
    let config: SramConfig = toml::from_str(&s)?;

    let cwd = cfg_path.parent().expect("invalid config file path");
    std::env::set_current_dir(cwd)?;

    log::info!("Beginning SRAM generation");
    generate(cwd.to_owned(), config, cli.netlist_only)?;

    Ok(())
}
