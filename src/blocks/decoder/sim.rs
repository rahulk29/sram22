use std::sync::Arc;

use super::layout::{DecoderStyle, PhysicalDesign, PhysicalDesignParams};
use super::{Decoder, DecoderParams, DecoderTree};
use crate::blocks::sram::WORDLINE_CAP_PER_CELL;
use serde::{Deserialize, Serialize};
use subgeom::Dir;
use substrate::component::Component;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpwl::Vpwl;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::Waveform;
use substrate::verification::simulation::{OutputFormat, TranAnalysis};

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DecoderCriticalPathTbParams {
    bits: usize,
    scale: i64,
    vdd: f64,
    period: f64,
    tr: f64,
    tf: f64,
}

pub struct DecoderCriticalPathTb {
    params: DecoderCriticalPathTbParams,
}

impl Component for DecoderCriticalPathTb {
    type Params = DecoderCriticalPathTbParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let vdd = ctx.signal("vdd");

        let params = &self.params;
        let addr = ctx.bus("addr", params.bits);
        let addr_b = ctx.bus("addr_b", params.bits);
        let decode = ctx.bus("decode", 2usize.pow(params.bits as u32));
        let decode_b = ctx.bus("decode_b", 2usize.pow(params.bits as u32));

        let vsupply = SiValue::with_precision(params.vdd, SiPrefix::Nano);
        ctx.instantiate::<Vdc>(&vsupply)?
            .named("Vdd")
            .with_connections([("p", vdd), ("n", vss)])
            .add_to(ctx);

        let tree = DecoderTree::new(params.bits, 64. * WORDLINE_CAP_PER_CELL);
        let decoder_params = DecoderParams {
            pd: PhysicalDesignParams {
                style: DecoderStyle::RowMatched,
                dir: Dir::Horiz,
            },
            max_width: None,
            tree,
        };
        ctx.instantiate::<Decoder>(&decoder_params)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("addr", addr),
                ("addr_b", addr_b),
                ("decode", decode),
                ("decode_b", decode_b),
            ])
            .named("Xdut")
            .add_to(ctx);

        let waveforms = self.waveforms();

        for i in 0..self.params.bits {
            ctx.instantiate::<Vpwl>(&waveforms.addr[i])?
                .named(format!("Vaddr[{i}]"))
                .with_connections([("p", addr.index(i)), ("n", vss)])
                .add_to(ctx);
            ctx.instantiate::<Vpwl>(&waveforms.addr_b[i])?
                .named(format!("Vaddr_b[{i}]"))
                .with_connections([("p", addr_b.index(i)), ("n", vss)])
                .add_to(ctx);
        }
        Ok(())
    }
}

impl Testbench for DecoderCriticalPathTb {
    type Output = ();
    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        let tran = TranAnalysis::builder()
            .start(0.0)
            .stop(self.t_stop())
            .step(self.params.period / 50.0)
            .build()
            .unwrap();
        ctx.add_analysis(tran);
        ctx.set_format(OutputFormat::DefaultViewable);
        Ok(())
    }

    fn measure(
        &mut self,
        _ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        Ok(())
    }
}

struct Waveforms {
    addr_b: Vec<Arc<Waveform>>,
    addr: Vec<Arc<Waveform>>,
}

impl DecoderCriticalPathTb {
    fn waveforms(&self) -> Waveforms {
        let params = &self.params;
        let n = params.bits;

        let mut out = Waveforms {
            addr: Vec::with_capacity(n),
            addr_b: Vec::with_capacity(n),
        };

        let t_stop = self.t_stop();
        for i in 0..n {
            let mut addr = Waveform::with_initial_value(0.0);
            let mut addr_b = Waveform::with_initial_value(params.vdd);

            let t_start = params.period / 4.0 + i as f64 * params.period;
            let t_end = t_start + params.period / 2.0;

            addr.push_low(t_start, params.vdd, params.tf);
            addr.push_high(t_end, params.vdd, params.tr);
            addr.push_low(t_stop, params.vdd, params.tf);

            addr_b.push_high(t_start, params.vdd, params.tr);
            addr_b.push_low(t_end, params.vdd, params.tf);
            addr_b.push_high(t_stop, params.vdd, params.tr);

            out.addr.push(Arc::new(addr));
            out.addr_b.push(Arc::new(addr_b));
        }

        assert_eq!(out.addr.len(), n);
        assert_eq!(out.addr_b.len(), n);

        out
    }

    #[inline]
    fn t_stop(&self) -> f64 {
        self.params.period * self.params.bits as f64
    }
}

#[cfg(test)]
mod tests {

    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    #[ignore = "slow"]
    fn test_decoder_critical_path_5bit() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_decoder_critical_path_5bit");

        let params = DecoderCriticalPathTbParams {
            bits: 5,
            scale: 1,
            vdd: 1.8,
            period: 20e-9,
            tr: 5e-12,
            tf: 5e-12,
        };

        ctx.write_simulation::<DecoderCriticalPathTb>(&params, &work_dir)
            .expect("failed to run simulation");
    }
}
