use std::io::Result;

fn main() -> Result<()> {
    let mut cfg = prost_build::Config::new();
    cfg.out_dir("./src/protos/");
    cfg.compile_protos(&["../protos/edatool/sim/v1/sim.proto"], &["../protos/"])?;
    Ok(())
}
