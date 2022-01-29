use sram22::{config::SramConfig, generate};

fn main() {
    let config = SramConfig {
        rows: 8,
        cols: 8,
        output_dir: "/home/rahul/acads/sky130/sram22/_build".to_string(),
        cell_dir: "/home/rahul/acads/sky130/sram22/tech/sky130/magic".to_string(),
    };

    generate(config).expect("failed to generate SRAM");
}
