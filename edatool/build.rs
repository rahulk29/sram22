use std::io::Result;

fn main() -> Result<()> {
    let mut cfg = prost_build::Config::new();

    // Add serde Serialize and Deserialize attributes
    cfg.type_attribute(".", "#[derive(::serde::Serialize, ::serde::Deserialize)]");

    // Place generated code in the source directory
    // Makes it easier to find the generated code when needed.
    cfg.out_dir("./src/protos/");

    // Compile the .proto sources
    cfg.compile_protos(&["../protos/edatool/sim/v1/sim.proto"], &["../protos/"])?;
    Ok(())
}
