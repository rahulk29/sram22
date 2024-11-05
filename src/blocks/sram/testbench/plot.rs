use crate::blocks::sram::testbench::{TbParams, TbSignals};
use plotters::backend::BitMapBackend;
use plotters::chart::ChartBuilder;
use plotters::drawing::IntoDrawingArea;
use plotters::element::PathElement;
use plotters::prelude::IntoFont;
use plotters::series::LineSeries;
use plotters::style::{Color, RGBColor, ShapeStyle};
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
    let data = std::fs::read(params.psf)?;
    let ast = psfparser::binary::parse(&data)?;
    let data = TransientData::from_binary(ast);
    let t = data.signal("time").unwrap();

    let root = BitMapBackend::new(&params.output_path, (1920, 1080)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(5)
        .caption("SRAM Read", ("sans-serif", 32.0).into_font())
        .build_cartesian_2d(138e-9f32..158e-9f32, -0.2f32..2.2f32)
        .unwrap();

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_desc("Time (ns)")
        .x_label_formatter(&|x| format!("{:.1}", x * 1e9f32))
        .y_desc("Voltage (V)")
        .draw()
        .unwrap();

    use plotters::style::colors::full_palette::*;
    let styles = [
        RED, PURPLE, INDIGO, BLUE, CYAN, TEAL, LIGHTGREEN, ORANGE, DEEPORANGE, BROWN, GREY,
        BLUEGREY,
    ];
    let mut styles = styles.into_iter().cycle();
    let mut plot = |name: &str, sig: TbSignals| {
        let style = styles.next().unwrap();
        let style = ShapeStyle {
            color: style.mix(1.0),
            filled: true,
            stroke_width: 3,
        };
        let y = data.signal(&params.tb.sram_signal_path(sig)).unwrap();
        chart
            .draw_series(LineSeries::new(
                t.iter().zip(y).map(|(x, y)| (*x as f32, *y as f32)),
                style.clone(),
            ))
            .unwrap()
            .label(name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], style.clone()));
    };

    plot("bl[4]", TbSignals::Bl(4));
    plot("br[4]", TbSignals::Br(4));
    plot("sae", TbSignals::SenseEnEnd);
    plot("pcb", TbSignals::PcBEnd);
    // plot("wlen", TbSignals::Wlen);
    // plot("wl[0]", TbSignals::WlEnd(0));
    plot("rbl", TbSignals::Rbl);
    plot("rwl", TbSignals::Rwl);
    // plot("clk", TbSignals::Clk);
    plot("dout[1]", TbSignals::Dout(1));

    chart
        .configure_series_labels()
        .background_style(RGBColor(192, 192, 192))
        .draw()
        .unwrap();

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file");
    println!("Result has been saved to {:?}", &params.output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::blocks::sram::testbench::plot::{plot_sim, PlotParams};
    use crate::blocks::sram::testbench::TestSequence;
    use crate::blocks::sram::tests::SRAM22_512X64M4W8;
    use crate::blocks::sram::SramPhysicalDesignScript;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use std::path::PathBuf;

    #[test]
    fn plot_sram() {
        let ctx = setup_ctx();
        let params = SRAM22_512X64M4W8;
        let seq = TestSequence::Short;
        let dsn = ctx
            .run_script::<SramPhysicalDesignScript>(&params)
            .expect("failed to run sram design script");
        let pex_level = calibre::pex::PexLevel::Rc;
        let sram_work_dir =
            PathBuf::from("/tools/C/rohankumar/sram22/build/test_sram22_512x64m4w8");
        let pex_netlist_path = crate::paths::out_pex(&sram_work_dir, "pex_netlist", pex_level);
        let pex_netlist = Some((pex_netlist_path.clone(), pex_level));
        let tb = crate::blocks::sram::testbench::tb_params(params, dsn, 1.8f64, seq, pex_netlist);
        let psf = sram_work_dir.join("tt_1.80_short/psf/analysis_0.tran.tran");

        let work_dir = test_work_dir("plot_sram");
        std::fs::create_dir_all(&work_dir).unwrap();
        let plot = PlotParams {
            tb,
            psf,
            output_path: work_dir.join("waveforms.png"),
        };
        plot_sim(plot).unwrap();
    }
}
