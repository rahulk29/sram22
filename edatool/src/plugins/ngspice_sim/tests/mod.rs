use std::path::PathBuf;

use approx::abs_diff_eq;

use crate::{
    protos::sim::{
        analysis_mode::Mode, Analysis, AnalysisMode, NamedExpression, OpParams, TranParams,
    },
    sim::{
        testbench::{NetlistSource, Testbench},
        waveform::{Waveform, WaveformBuf},
    },
};

use super::Ngspice;

#[test]
fn test_ngspice_vdivider() -> Result<(), Box<dyn std::error::Error>> {
    let netlist = test_data_path().join("vdivider.spice");
    let tb = Testbench::with_source(NetlistSource::File(netlist));
    let mut ngs = Ngspice::with_tb(tb);
    ngs.cwd("/tmp/sram22/tests/sim/vdivider".into());
    let op = Analysis {
        mode: Some(AnalysisMode {
            mode: Some(Mode::Op(OpParams {})),
        }),
        expressions: vec![NamedExpression {
            name: "out".to_string(),
            expr: "v(out)".to_string(),
        }],
    };

    ngs.add_analysis(op)?;
    let mut data = ngs.run()?;

    let x = data.analyses[0].values.remove("out").unwrap().unwrap_real();
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
    let op = Analysis {
        mode: Some(AnalysisMode {
            mode: Some(Mode::Op(OpParams {})),
        }),
        expressions: vec![NamedExpression {
            name: "out".to_string(),
            expr: "v(out)".to_string(),
        }],
    };

    ngs.add_analysis(op)?;
    let mut data = ngs.run()?;

    let x = data.analyses[0].values.remove("out").unwrap().unwrap_real();
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
    let tran = Analysis {
        mode: Some(AnalysisMode {
            mode: Some(Mode::Tran(TranParams {
                tstop: 4f64,
                tstep: 1f64,
                tstart: 0f64,
                uic: false,
            })),
        }),
        expressions: vec![NamedExpression {
            name: "out".to_string(),
            expr: "v(out)".to_string(),
        }],
    };

    // Set up ngspice
    let mut ngs = Ngspice::with_tb(tb);
    ngs.cwd("/tmp/sram22/tests/sim/vdivider_tran".into());
    ngs.add_analysis(tran)?;
    let mut data = ngs.run()?;

    let t = data.analyses[0]
        .values
        .remove("sweep_var")
        .unwrap()
        .unwrap_real();
    let y = data.analyses[0].values.remove("out").unwrap().unwrap_real();

    let wav = Waveform::new(&t, &y);

    println!("got data from tran simulation:");
    println!("{}", wav);

    Ok(())
}

fn test_data_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/plugins/ngspice_sim/tests/data")
}
