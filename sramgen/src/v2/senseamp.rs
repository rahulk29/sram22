use std::sync::Arc;

use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::capacitor::Capacitor;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpwl::Vpwl;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::bits::is_logical_low;
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::{TimeWaveform, Waveform};
use substrate::verification::simulation::TranAnalysis;

use super::macros::SenseAmp;

pub struct OffsetTb {
    params: OffsetTbParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offset {
    value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetTbParams {
    vnom: f64,
    vdd: f64,
    vincr: f64,
    period: f64,
    tslew: f64,
    n_incr: usize,
    cout: SiValue,
}

impl OffsetTb {
    fn waveforms(&self) -> (Waveform, Waveform) {
        let vdd = self.params.vdd;
        let ts = self.params.tslew;
        let period = self.params.period;
        let vnom = self.params.vnom;
        let vincr = self.params.vincr;
        let mut clk = Waveform::with_initial_value(0.0);
        let mut vin = Waveform::with_initial_value(vnom);

        for i in 0..self.params.n_incr + 1 {
            let t = i as f64 * period;
            let t_negedge = t + period / 2.0;
            let t_posedge = t + period;
            clk.push_high(t_negedge, vdd, ts);
            clk.push_low(t_posedge, vdd, ts);
            vin.push(t_negedge, vin.last_x().unwrap());

            let value = vnom - i as f64 * vincr;
            vin.push(t_negedge + ts, value);
        }

        (vin, clk)
    }
}

impl Component for OffsetTb {
    type Params = OffsetTbParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sense_amp_offset_tb")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let [clk, inn, inp, outn, outp, vdd] =
            ctx.signals(["clk", "inn", "inp", "outn", "outp", "vdd"]);

        let mut coutp = ctx.instantiate::<Capacitor>(&self.params.cout)?;
        coutp.connect_all([("p", outp), ("n", vss)]);
        coutp.set_name("coutp");
        ctx.add_instance(coutp);

        let mut coutn = ctx.instantiate::<Capacitor>(&self.params.cout)?;
        coutn.connect_all([("p", outn), ("n", vss)]);
        coutn.set_name("coutn");
        ctx.add_instance(coutn);

        let mut vnom =
            ctx.instantiate::<Vdc>(&SiValue::with_precision(self.params.vnom, SiPrefix::Micro))?;
        vnom.connect_all([("p", inn), ("n", vss)]);
        vnom.set_name("vnom");
        ctx.add_instance(vnom);

        let vvdd = ctx
            .instantiate::<Vdc>(&SiValue::with_precision(self.params.vdd, SiPrefix::Micro))?
            .with_connections([("p", vdd), ("n", vss)])
            .named("vvdd");
        ctx.add_instance(vvdd);

        let (vin, vclk) = self.waveforms();
        let vclk = ctx
            .instantiate::<Vpwl>(&Arc::new(vclk))?
            .with_connections([("p", clk), ("n", vss)])
            .named("vclk");
        ctx.add_instance(vclk);

        let vvin = ctx
            .instantiate::<Vpwl>(&Arc::new(vin))?
            .with_connections([("p", inp), ("n", vss)])
            .named("vvin");
        ctx.add_instance(vvin);

        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.connect_all([
            ("clk", clk),
            ("inn", inn),
            ("inp", inp),
            ("outn", outn),
            ("outp", outp),
            ("VDD", vdd),
            ("VSS", vss),
        ]);
        sa.set_name("dut");
        ctx.add_instance(sa);

        Ok(())
    }
}

impl Testbench for OffsetTb {
    type Output = Offset;

    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        ctx.add_analysis(TranAnalysis {
            start: 0.0,
            stop: self.params.period * (self.params.n_incr + 1) as f64,
            step: self.params.period / 40.0,
        })
        .save(substrate::verification::simulation::Save::All);
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        let data = &ctx.output().data[0].tran();
        let vout = &data.data["v(xdut.outp)"];
        let vinp = &data.data["v(xdut.inp)"];
        let t = &data.time;

        let period = self.params.period;

        let mut idx_thresh = None;
        for i in 0..self.params.n_incr + 1 {
            let t_i = i as f64 * period;
            let t_negedge = t_i + period / 2.0;

            let idx = t
                .where_at_least(t_negedge - self.params.period / 10.0)
                .unwrap();
            let vout = vout.values[idx];
            if is_logical_low(vout, self.params.vdd) {
                idx_thresh = Some(idx);
                break;
            }
        }

        let idx_thresh = idx_thresh.unwrap();
        let ofs = self.params.vnom - vinp.values[idx_thresh];
        assert!(ofs >= 0.0, "offset {} expected to be larger than 0", ofs);
        Ok(Offset { value: ofs })
    }
}

#[cfg(test)]
mod tests {
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    #[ignore = "slow"]
    fn test_sa_offset_tb() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sa_offset_tb");
        let params = OffsetTbParams {
            vnom: 1.7,
            vdd: 1.8,
            vincr: 0.1e-3,
            period: 4e-9,
            tslew: 10e-12,
            n_incr: 1_000,
            cout: SiValue::new(2, SiPrefix::Femto),
        };
        let offset = ctx
            .write_simulation::<OffsetTb>(&params, &work_dir)
            .expect("failed to run simulation");
        println!("SA offset = {:?}", offset);
    }
}
