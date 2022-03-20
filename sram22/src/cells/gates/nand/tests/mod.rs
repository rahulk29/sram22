use crate::{cells::gates::GateSize, sky130_config, tech_spice_include, test_utils::get_magic};
use edatool::{
    pex::{Pex, PexInput, PexOpts},
    plugins::{magic_pex::MagicPex, ngspice_sim::Ngspice},
    protos::sim::{analysis_mode::Mode, Analysis, AnalysisMode, NamedExpression, TranParams},
    sim::{
        testbench::{NetlistSource, Testbench},
        waveform::{Waveform, WaveformBuf},
    },
};
use std::{
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use super::{
    single_height::{generate_pm_single_height, Nand2Params},
    Nand2Gate,
};
use magic_vlsi::units::Distance;
use micro_hdl::{backend::spice::SpiceBackend, frontend::parse};

#[test]
fn test_netlist_nand2() -> Result<(), Box<dyn std::error::Error>> {
    let tree = parse(Nand2Gate::top(GateSize::minimum()));
    let file = tempfile::tempfile()?;
    let mut backend = SpiceBackend::with_file(file)?;
    backend.netlist(&tree)?;
    let mut file = backend.output();

    let mut s = String::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_string(&mut s)?;
    println!("{}", &s);

    Ok(())
}

#[test]
fn test_simulate_pex_nand2() -> Result<(), Box<dyn std::error::Error>> {
    let work_dir: PathBuf = "/tmp/sram22/tests/sim/nand2".into();
    let tc = sky130_config();
    let mut m = get_magic();

    let cell_name = generate_pm_single_height(
        &mut m,
        &tc,
        &Nand2Params {
            nmos_scale: Distance::from_nm(1_000),
            height: Distance::from_nm(1_580),
        },
    )?;
    let layout_path = m.getcwd().join(format!("{}.mag", cell_name));

    let output = MagicPex::pex(PexInput {
        layout: layout_path,
        layout_cell: cell_name.clone(),
        work_dir: work_dir.clone(),
        tech: "sky130A".to_string(),
        opts: PexOpts {},
    })?;

    // a b gnd pwr y vpb
    let tb = format!(
        "Vdd vpwr 0 dc 1.8
    Ava %vd([a 0]) wav_a
    Avb %vd([b 0]) wav_b
    Xnand a b 0 vpwr y vpwr {}
    .model wav_a filesource (file=\"va.m\" amploffset=[0 0] amplscale=[1 1] amplstep=true)
    .model wav_b filesource (file=\"vb.m\" amploffset=[0 0] amplscale=[1 1] amplstep=true)
    ",
        &cell_name
    );

    let mut tb = Testbench::with_source(NetlistSource::Str(tb));
    tb.add_named_lib(tech_spice_include(), "tt".to_string());
    tb.include(output.netlist);

    let vdd = 1.8f64;

    let t = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let va = vec![0.0, 0.0, vdd, vdd, vdd];
    let vb = vec![0.0, vdd, 0.0, vdd, vdd];

    let va = WaveformBuf::with_named_data("va.m", &t, &va);
    let vb = WaveformBuf::with_named_data("vb.m", &t, &vb);

    tb.add_waveform(va);
    tb.add_waveform(vb);

    let tran = Analysis {
        mode: Some(AnalysisMode {
            mode: Some(Mode::Tran(TranParams {
                tstep: 1_E-4f64,
                tstop: 4f64,
                ..Default::default()
            })),
        }),
        expressions: vec![
            NamedExpression {
                name: "a".to_string(),
                expr: "v(a)".to_string(),
            },
            NamedExpression {
                name: "b".to_string(),
                expr: "v(b)".to_string(),
            },
            NamedExpression {
                name: "y".to_string(),
                expr: "v(y)".to_string(),
            },
        ],
    };

    let mut ngs = Ngspice::with_tb(tb);
    ngs.cwd(work_dir);
    ngs.add_analysis(tran)?;
    let mut data = ngs.run()?;

    // Save data to a file:
    // data.to_file("/home/rahul/acads/sky130/sram22/scripts/simdata.bin")?;

    let t = data.analyses[0]
        .values
        .remove("sweep_var")
        .unwrap()
        .unwrap_real();
    let y = data.analyses[0].values.remove("y").unwrap().unwrap_real();
    let wav = Waveform::new(&t, &y);

    println!("got data from tran simulation:");
    println!("{}", wav);

    Ok(())
}
