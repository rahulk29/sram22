use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpulse::Vpulse;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::{OutputFormat, TranAnalysis};

use super::*;

pub struct DelayLineTb {
    params: DelayLineTbParams,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelayLineTbParams {
    pub inner: DelayLineParams,
    pub vdd: f64,
    /// Rise time.
    pub tr: f64,
    /// Clock frequency.
    pub f: f64,
    /// Time at each delay setting.
    pub ctl_period: f64,
    /// Simulation end time.
    ///
    /// Defaults to `inner.stages * ctl_period`.
    pub t_stop: Option<f64>,
}

impl Component for DelayLineTb {
    type Params = DelayLineTbParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("delay_line_testbench")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let stages = self.params.inner.stages;

        let vss = ctx.port("vss", Direction::InOut);
        let [vdd, clk_in, clk_out] = ctx.signals(["vdd", "clk_in", "clk_out"]);
        let ctl = ctx.bus("ctl", stages);
        let ctl_b = ctx.bus("ctl_b", stages);

        let vmax = SiValue::with_precision(self.params.vdd, SiPrefix::Nano);
        ctx.instantiate::<Vdc>(&vmax)?
            .with_connections([("p", vdd), ("n", vss)])
            .named("Vvdd")
            .add_to(ctx);

        let clk_period = SiValue::with_precision(1. / self.params.f, SiPrefix::Femto);
        let half_clk_period = SiValue::with_precision(1. / 2. / self.params.f, SiPrefix::Femto);
        let tr = SiValue::with_precision(self.params.tr, SiPrefix::Femto);
        let ctl_period = SiValue::with_precision(self.params.ctl_period, SiPrefix::Femto);
        let anti_ctl_period = SiValue::with_precision(
            (stages - 1) as f64 * self.params.ctl_period,
            SiPrefix::Femto,
        );
        let all_ctl_period =
            SiValue::with_precision(stages as f64 * self.params.ctl_period, SiPrefix::Femto);

        ctx.instantiate::<Vpulse>(&Vpulse {
            v1: SiValue::zero(),
            v2: vmax,
            td: SiValue::zero(),
            tr,
            tf: tr,
            pw: half_clk_period,
            period: clk_period,
        })?
        .with_connections([("p", clk_in), ("n", vss)])
        .named("Vclk")
        .add_to(ctx);

        for i in 0..stages{
            for j in 0..2 {
                ctx.instantiate::<Vpulse>(&Vpulse {
                    v1: SiValue::zero(),
                    v2: vmax,
                    td: SiValue::with_precision(
                        (i as f64 + j as f64 - stages as f64) * self.params.ctl_period,
                        SiPrefix::Femto,
                    ),
                    tr,
                    tf: tr,
                    pw: if j == 0 { ctl_period } else { anti_ctl_period },
                    period: all_ctl_period,
                })?
                .with_connections([("p", if j == 0 { ctl } else { ctl_b }.index(i)), ("n", vss)])
                .named("Va")
                .add_to(ctx);
            }
        }

        ctx.instantiate::<DelayLine>(&self.params.inner)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("clk_in", clk_in),
                ("clk_out", clk_out),
                ("ctl", ctl),
                ("ctl_b", ctl_b),
            ])
            .named("Xdut")
            .add_to(ctx);

        Ok(())
    }
}

impl Testbench for DelayLineTb {
    type Output = ();
    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        let tran = TranAnalysis::builder()
            .start(0.0)
            .stop(
                self.params
                    .t_stop
                    .unwrap_or(self.params.inner.stages as f64 * self.params.ctl_period),
            )
            .step(1. / 10. / self.params.f)
            .build()
            .unwrap();
        ctx.add_analysis(tran);
        ctx.set_format(OutputFormat::DefaultViewable);
        ctx.save(substrate::verification::simulation::Save::All);
        Ok(())
    }

    fn measure(
        &mut self,
        _ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        Ok(())
    }
}
