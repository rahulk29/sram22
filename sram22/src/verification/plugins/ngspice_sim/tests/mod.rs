use std::path::PathBuf;

use approx::abs_diff_eq;

use crate::verification::sim::{
    analysis::Analysis,
    testbench::{NetlistSource, Testbench},
};

use super::Ngspice;

#[test]
fn test_ngspice_vdivider() -> Result<(), Box<dyn std::error::Error>> {
    let netlist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/verification/plugins/ngspice_sim/tests/data/vdivider.spice");
    let tb = Testbench::with_source(NetlistSource::File(netlist));
    let mut ngs = Ngspice::with_tb(tb);
    ngs.cwd("/tmp/sram22/tests/sim/vdivider".into());
    let mut op = Analysis::with_mode(crate::verification::sim::analysis::Mode::Op);
    op.save("v(out)".to_string());

    ngs.add_analysis(op);
    let mut data = ngs.run()?;

    let x = data.analyses[0].data.remove("v(out)").unwrap().real();
    assert_eq!(x.len(), 1);
    assert!(abs_diff_eq!(x[0], 0.5f64));

    println!("done");
    Ok(())
}
