use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["../protos/edatool/sim/v1/sim.proto"], &["../protos/"])?;
    Ok(())
}
