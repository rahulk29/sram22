use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &["../protos/sram22/verification/sim/v1/sim.proto"],
        &["../protos/"],
    )?;
    Ok(())
}
