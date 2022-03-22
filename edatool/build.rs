use std::io::Result;

fn main() -> Result<()> {
    let mut cfg = prost_build::Config::new();

    // Add serde Serialize and Deserialize attributes
    cfg.type_attribute(".", "#[derive(::serde::Serialize, ::serde::Deserialize)]");

    // Place generated code in the source directory
    // Makes it easier to find the generated code when needed.
    cfg.out_dir("./src/protos/");

    let srcs = [
        "../protos/edatool/sim/v1/sim.proto",
        "../protos/edatool/lvs/v1/lvs.proto",
    ];

    // Compile the .proto sources
    cfg.compile_protos(&srcs, &["../protos/"])?;
    Ok(())
}
