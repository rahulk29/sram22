use crate::blocks::sram::testbench::{TbParams, TbSignals};
use psfparser::analysis::transient::TransientData;
use psfparser::binary::ast::PsfAst;
use std::path::PathBuf;
use substrate::verification::simulation::waveform::SharedWaveform;

pub struct PlotParams {
    tb: TbParams,
    psf: PathBuf,
    output_path: PathBuf,
}

pub fn plot_sim(params: PlotParams) -> substrate::error::Result<()> {
    use plotters::prelude::*;

    let data = std::fs::read(params.psf)?;
    let ast = psfparser::binary::parse(&data)?;
    let data = TransientData::from_binary(ast);
    let t = data.signal("time").unwrap();
    let y = data
        .signal(&params.tb.sram_signal_path(TbSignals::Bl(0)))
        .unwrap();

    let root = BitMapBackend::new(&params.output_path, (1920, 1080)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(5)
        .caption("Dual Y-Axis Example", ("sans-serif", 50.0).into_font())
        .build_cartesian_2d(0f32..20e-9f32, -0.2f32..2.2f32)
        .unwrap();

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .y_desc("Voltage")
        .draw()
        .unwrap();

    chart
        .draw_series(LineSeries::new(
            t.iter().zip(y).map(|(x, y)| (*x as f32, *y as f32)),
            &BLUE,
        ))
        .unwrap()
        .label("bl[0]")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart
        .configure_series_labels()
        .background_style(RGBColor(128, 128, 128))
        .draw()
        .unwrap();

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file");
    println!("Result has been saved to {:?}", &params.output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::blocks::sram::testbench::plot::PlotParams;
    use crate::blocks::sram::testbench::TestSequence;
    use crate::blocks::sram::tests::SRAM22_512X64M4W8;
    use crate::blocks::sram::SramPhysicalDesignScript;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    #[test]
    fn plot_sram() {
        let ctx = setup_ctx();
        let params = SRAM22_512X64M4W8;
        let seq = TestSequence::Short;
        let dsn = ctx
            .run_script::<SramPhysicalDesignScript>(&params)
            .expect("failed to run sram design script");
        let pex_level = calibre::pex::PexLevel::Rc;
        let sram_work_dir = test_work_dir("test_sram22_512x64m4w8");
        let pex_netlist_path = crate::paths::out_pex(&sram_work_dir, "pex_netlist", pex_level);
        let pex_netlist = Some((pex_netlist_path.clone(), pex_level));
        let tb = crate::blocks::sram::testbench::tb_params(params, dsn, 1.8f64, seq, pex_netlist);
        let psf = sram_work_dir.join("tt_1.80_short/psf");

        let work_dir = test_work_dir("plot_sram");
        let plot = PlotParams {
            tb,
            psf,
            output_path: work_dir.join("waveforms.png"),
        };
    }
}
