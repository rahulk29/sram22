use std::path::PathBuf;

use super::Ngspice;

#[test]
fn test_ngspice_vdivider() -> Result<(), Box<dyn std::error::Error>> {
    let netlist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/verification/plugins/ngspice_sim/tests/data/vdivider.spice");
    println!("netlist: {:?}", &netlist);
    let mut ngs = Ngspice::builder()
        .netlist(netlist)
        .cwd(PathBuf::from("/tmp/devsram22/"))
        .build()?;
    ngs.send("op")?;
    ngs.send("wrdata /tmp/devsram22/outdata v(out)")?;
    ngs.flush()?;
    println!("done");
    Ok(())
}
