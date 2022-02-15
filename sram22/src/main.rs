use sram22::{config::SramConfig, generate};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let config = SramConfig {
        rows: 16,
        cols: 16,
        output_dir: "/home/rahul/acads/sky130/sram22/_build".to_string(),
        cell_dir: "/home/rahul/acads/sky130/sram22/tech/sky130/magic".to_string(),
    };

    generate(config).expect("failed to generate SRAM");
}
