use std::path::PathBuf;

use approx::abs_diff_eq;

use crate::sim::{
    analysis::{Analysis, TransientAnalysis},
    testbench::{NetlistSource, Testbench},
    waveform::{Waveform, WaveformBuf},
};

use super::Ngspice;

#[test]
fn test_ngspice_vdivider() -> Result<(), Box<dyn std::error::Error>> {
    let netlist = test_data_path().join("vdivider.spice");
    let tb = Testbench::with_source(NetlistSource::File(netlist));
    let mut ngs = Ngspice::with_tb(tb);
    ngs.cwd("/tmp/sram22/tests/sim/vdivider".into());
    let mut op = Analysis::with_mode(crate::sim::analysis::Mode::Op);
    op.save("v(out)");

    ngs.add_analysis(op);
    let mut data = ngs.run()?;

    let x = data.analyses[0].data.remove("v(out)").unwrap().real();
    assert_eq!(x.len(), 1);
    assert!(abs_diff_eq!(x[0], 0.5f64));

    Ok(())
}

#[test]
fn test_ngspice_include1() -> Result<(), Box<dyn std::error::Error>> {
    let netlist = test_data_path().join("include1.spice");
    let include = test_data_path().join("vdivider.spice");

    let mut tb = Testbench::with_source(NetlistSource::File(netlist));
    tb.include(include);
    let mut ngs = Ngspice::with_tb(tb);
    ngs.cwd("/tmp/sram22/tests/sim/include1".into());
    let mut op = Analysis::with_mode(crate::sim::analysis::Mode::Op);
    op.save("v(out)");

    ngs.add_analysis(op);
    let mut data = ngs.run()?;

    let x = data.analyses[0].data.remove("v(out)").unwrap().real();
    assert_eq!(x.len(), 1);
    assert!(abs_diff_eq!(x[0], 0.5f64));

    Ok(())
}

#[test]
fn test_vdivider_tran() -> Result<(), Box<dyn std::error::Error>> {
    // Set up testbench
    let netlist = test_data_path().join("vdivider_tran.spice");
    let mut tb = Testbench::with_source(NetlistSource::File(netlist));
    let t = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let v = vec![2.0, 1.0, 2.0, 1.0, 2.0];
    let wav = WaveformBuf::with_named_data("input.m", &t, &v);
    tb.add_waveform(wav);

    // Set up analysis
    let mut tran = Analysis::with_mode(crate::sim::analysis::Mode::Tran(TransientAnalysis {
        tstart: 0f64,
        tstep: 1f64,
        tstop: 4f64,
        uic: false,
    }));
    tran.save("v(out)");

    // Set up ngspice
    let mut ngs = Ngspice::with_tb(tb);
    ngs.cwd("/tmp/sram22/tests/sim/vdivider_tran".into());
    ngs.add_analysis(tran);
    let mut data = ngs.run()?;

    let t = data.analyses[0].data.remove("sweep_var").unwrap().real();
    let y = data.analyses[0].data.remove("v(out)").unwrap().real();
    let wav = Waveform::new(&t, &y);

    println!("got data from tran simulation:");
    println!("{}", wav);

    Ok(())
}

fn test_data_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/plugins/ngspice_sim/tests/data")
}
