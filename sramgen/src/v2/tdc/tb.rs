use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpulse::Vpulse;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::{OutputFormat, TranAnalysis};

use super::*;

pub struct TdcTb {
    params: TdcTbParams,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TdcTbParams {
    pub inner: TdcParams,
    pub vdd: f64,
    /// Difference between input waveform rising edges.
    pub delta_t: f64,
    /// Rise time.
    pub tr: f64,
    /// Simulation end time.
    pub t_stop: f64,
}

impl Component for TdcTb {
    type Params = TdcTbParams;
    fn new(
        params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tdc_testbench")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let [vdd, a, b] = ctx.signals(["vdd", "a", "b"]);
        let dout = ctx.bus("dout", self.params.inner.bits_out());

        let vmax = SiValue::with_precision(self.params.vdd, SiPrefix::Nano);
        ctx.instantiate::<Vdc>(&vmax)?
            .with_connections([("p", vdd), ("n", vss)])
            .named("Vvdd")
            .add_to(ctx);

        let t0 = 100e-12;
        let ta = SiValue::with_precision(t0, SiPrefix::Femto);
        let tb = SiValue::with_precision(self.params.delta_t + t0, SiPrefix::Femto);
        let tr = SiValue::with_precision(self.params.tr, SiPrefix::Femto);

        ctx.instantiate::<Vpulse>(&Vpulse {
            v1: SiValue::zero(),
            v2: vmax,
            td: ta,
            tr,
            tf: tr,
            pw: SiValue::new(1000, SiPrefix::None),
            period: SiValue::new(2000, SiPrefix::None),
        })?
        .with_connections([("p", a), ("n", vss)])
        .named("Va")
        .add_to(ctx);

        ctx.instantiate::<Vpulse>(&Vpulse {
            v1: SiValue::zero(),
            v2: vmax,
            td: tb,
            tr,
            tf: tr,
            pw: SiValue::new(1000, SiPrefix::None),
            period: SiValue::new(2000, SiPrefix::None),
        })?
        .with_connections([("p", b), ("n", vss)])
        .named("Vb")
        .add_to(ctx);

        ctx.instantiate::<Tdc>(&self.params.inner)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("a", a),
                ("b", b),
                ("dout", dout),
            ])
            .named("Xdut")
            .add_to(ctx);

        Ok(())
    }
}

impl Testbench for TdcTb {
    type Output = ();
    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        let tran = TranAnalysis::builder()
            .start(0.0)
            .stop(self.params.t_stop)
            .step(self.params.delta_t / 10.0)
            .build()
            .unwrap();
        ctx.add_analysis(tran);
        ctx.set_format(OutputFormat::DefaultViewable);
        ctx.save(substrate::verification::simulation::Save::All);
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        Ok(())
    }
}
