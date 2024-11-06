use crate::blocks::sram::testbench::{TbParams, TbSignals};
use plotters::backend::BitMapBackend;
use plotters::chart::ChartBuilder;
use plotters::coord::types::RangedCoordf32;
use plotters::drawing::IntoDrawingArea;
use plotters::element::PathElement;
use plotters::prelude::IntoFont;
use plotters::series::LineSeries;
use plotters::style::{Color, RGBColor, ShapeStyle};
use psfparser::analysis::transient::TransientData;
use psfparser::binary::ast::PsfAst;
use std::ops::Range;
use std::path::PathBuf;
use substrate::verification::simulation::waveform::SharedWaveform;

#[derive(Debug, Clone)]
pub struct PlotParams {
    tb: TbParams,
    psf: PathBuf,
    output_path: PathBuf,
    plot_name: String,
}

fn plot_inner(
    params: PlotParams,
    time_span: Range<f32>,
    signals: &[(&str, TbSignals)],
) -> substrate::error::Result<()> {
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
        .caption(params.plot_name, ("sans-serif", 32.0).into_font())
        .build_cartesian_2d(time_span, -0.2f32..2.2f32)
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

    for (name, sig) in signals {
        plot(name, sig.clone());
    }
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

pub fn plot_read(params: PlotParams) -> substrate::error::Result<()> {
    plot_inner(
        params,
        138e-9f32..158e-9f32,
        &vec![
            ("clk", TbSignals::Clk),
            ("rwl", TbSignals::Rwl),
            ("rbl", TbSignals::Rbl),
            ("bl[4]", TbSignals::Bl(8)),
            ("br[4]", TbSignals::Br(8)),
            ("sae", TbSignals::SenseEnEnd),
            ("dout[1]", TbSignals::Dout(1)),
            ("pcb", TbSignals::PcBEnd),
        ],
    )
}

pub fn plot_write(params: PlotParams) -> substrate::error::Result<()> {
    plot_inner(
        params,
        38e-9f32..58e-9f32,
        &vec![
            ("clk", TbSignals::Clk),
            ("write_driver_en[0]", TbSignals::WeI(0)),
            ("write_driver_enb[0]", TbSignals::WeIb(0)),
            ("wl[0]", TbSignals::WlEnd(0)),
            ("bl[4]", TbSignals::Bl(8)),
            ("br[4]", TbSignals::Br(8)),
            ("pcb", TbSignals::PcBEnd),
        ],
    )
}

#[cfg(test)]
mod tests {
    use crate::blocks::sram::testbench::plot::*;
    use crate::blocks::sram::testbench::TestSequence;
    use crate::blocks::sram::tests::*;
    use crate::blocks::sram::SramPhysicalDesignScript;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use std::path::PathBuf;

    #[test]
    fn plot_sram() {
        let ctx = setup_ctx();
        let seq = TestSequence::Short;
        for params in [
            SRAM22_64X24M4W8,
            SRAM22_64X32M4W8,
            SRAM22_128X16M4W8,
            SRAM22_128X24M4W8,
            SRAM22_128X32M4W8,
            SRAM22_256X8M8W1,
            SRAM22_256X16M8W8,
            SRAM22_256X32M4W8,
            SRAM22_256X64M4W8,
            SRAM22_256X128M4W8,
            SRAM22_512X8M8W1,
            SRAM22_512X32M4W8,
            SRAM22_512X64M4W8,
            SRAM22_512X128M4W8,
            SRAM22_1024X8M8W1,
            SRAM22_1024X32M8W8,
            SRAM22_1024X64M4W8,
            SRAM22_2048X8M8W1,
            SRAM22_2048X32M8W8,
        ] {
            let dsn = ctx
                .run_script::<SramPhysicalDesignScript>(&params)
                .expect("failed to run sram design script");
            let pex_level = calibre::pex::PexLevel::Rc;
            let sram_work_dir = PathBuf::from(format!(
                "/tools/C/rohankumar/sram22/build/test_{}",
                params.name()
            ));
            let pex_netlist_path = crate::paths::out_pex(&sram_work_dir, "pex_netlist", pex_level);
            let pex_netlist = Some((pex_netlist_path.clone(), pex_level));
            let tb =
                crate::blocks::sram::testbench::tb_params(params, dsn, 1.8f64, seq, pex_netlist);
            for corner in ["sf", "fs", "ss", "ff"] {
                let psf =
                    sram_work_dir.join(format!("{corner}_1.80_short/psf/analysis_0.tran.tran"));

                let work_dir = test_work_dir("plot_sram");
                std::fs::create_dir_all(&work_dir).unwrap();
                let plot = PlotParams {
                    tb: tb.clone(),
                    psf: psf.clone(),
                    output_path: work_dir.join(format!("{}_{}_read.png", params.name(), corner)),
                    plot_name: format!(
                        "{} read (RC extracted, {}/25C/1.8V)",
                        params.name(),
                        corner.to_uppercase()
                    ),
                };
                plot_read(plot).unwrap();
                let plot = PlotParams {
                    tb: tb.clone(),
                    psf: psf.clone(),
                    output_path: work_dir.join(format!("{}_{}_write.png", params.name(), corner)),
                    plot_name: format!(
                        "{} write (RC extracted, {}/25C/1.8V)",
                        params.name(),
                        corner.to_uppercase()
                    ),
                };
                plot_write(plot.clone()).unwrap();
            }
        }
    }
}
