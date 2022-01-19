use sram22::{config::SramConfig, generate_32x64};

fn main() {
    let config = SramConfig {
        rows: 64,
        cols: 32,
        output_dir: "/home/rahul/acads/sky130/sram22/_build".to_string(),
        cell_dir: "/home/rahul/acads/sky130/sram22/tech/sky130/magic".to_string(),
    };

    generate_32x64(config).expect("failed to generate SRAM");
}
