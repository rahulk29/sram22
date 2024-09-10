use std::collections::HashMap;
use std::path::PathBuf;

use arcstr::ArcStr;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use substrate::component::{error, Component};
use substrate::error::ErrorSource;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::idc::Idc;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpulse::Vpulse;
use substrate::schematic::signal::Signal;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::{SharedWaveform, TimeWaveform};
use substrate::verification::simulation::{Analysis, Save, TranAnalysis};

use crate::pex::Pex;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum TransitionTbNode {
    Vdd,
    Vss,
    // Node to be measured.
    Vmeas,
    // Node to apply stimulus.
    Vstim,
    Floating,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionTimes {
    pub tr: f64,
    pub tf: f64,
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct TransitionTbParams<T> {
    /// Supply voltage.
    pub vdd: f64,
    pub delay: f64,
    pub width: f64,
    pub fall: f64,
    pub rise: f64,
    pub upper_threshold: f64,
    pub lower_threshold: f64,
    pub dut: T,
    pub pex_netlist: Option<PathBuf>,
    pub connections: HashMap<ArcStr, Vec<TransitionTbNode>>,
}

impl<T: Clone> TransitionTbParams<T> {
    #[inline]
    pub fn builder() -> TransitionTbParamsBuilder<T> {
        TransitionTbParamsBuilder::default()
    }
}

pub struct TransitionTestbench<T: Component> {
    params: TransitionTbParams<T::Params>,
}

impl<P: Clone + Serialize, T: Component<Params = P>> Component for TransitionTestbench<T> {
    type Params = TransitionTbParams<T::Params>;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("transition_testbench")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let vdd = ctx.signal("vdd");
        let vmeas = ctx.signal("vmeas");
        let vstim = ctx.signal("vstim");
        let mut ctr = 0;

        let mut connections = Vec::new();

        for (k, nodes) in &self.params.connections {
            let mut signal = Vec::new();
            for node in nodes {
                signal.push(match node {
                    TransitionTbNode::Vdd => vdd,
                    TransitionTbNode::Vss => vss,
                    TransitionTbNode::Vmeas => vmeas,
                    TransitionTbNode::Vstim => vstim,
                    TransitionTbNode::Floating => {
                        ctr += 1;
                        ctx.signal(format!("floating{ctr}"))
                    }
                });
            }
            connections.push((k.clone(), Signal::new(signal)));
        }

        if self.params.pex_netlist.is_some() {
            ctx.instantiate::<Pex<T>>(&self.params.dut)?
                .with_connections(connections)
                .named("dut")
                .add_to(ctx);
        } else {
            ctx.instantiate::<T>(&self.params.dut)?
                .with_connections(connections)
                .named("dut")
                .add_to(ctx);
        }

        ctx.instantiate::<Vdc>(&SiValue::with_precision(self.params.vdd, SiPrefix::Milli))?
            .with_connections([("p", vdd), ("n", vss)])
            .named("Vdd")
            .add_to(ctx);

        ctx.instantiate::<Vpulse>(&Vpulse {
            v1: SiValue::zero(),
            v2: SiValue::with_precision(self.params.vdd, SiPrefix::Milli),
            td: SiValue::with_precision(self.params.delay, SiPrefix::Pico),
            tr: SiValue::with_precision(self.params.rise, SiPrefix::Pico),
            tf: SiValue::with_precision(self.params.fall, SiPrefix::Pico),
            pw: SiValue::with_precision(self.params.width, SiPrefix::Pico),
            period: SiValue::zero(),
        })?
        .with_connections([("p", vstim), ("n", vss)])
        .named("Vstim")
        .add_to(ctx);

        Ok(())
    }
}

// impl<P: Clone + Serialize, T: Component<Params = P>> Testbench for TransitionTestbench<T> {
//     type Output = TransitionTimes;
//
//     fn setup(
//         &mut self,
//         ctx: &mut substrate::verification::simulation::context::PreSimCtx,
//     ) -> substrate::error::Result<()> {
//         if let Some(ref netlist) = self.params.pex_netlist {
//             ctx.include(netlist);
//         }
//         ctx.add_analysis(Analysis::Tran(
//             TranAnalysis::builder()
//                 .stop(6e-6)
//                 .start(0.0)
//                 .step(1e-9)
//                 .build()
//                 .unwrap(),
//         ))
//         .save(Save::All);
//         Ok(())
//     }
//
//     fn measure(
//         &mut self,
//         ctx: &substrate::verification::simulation::context::PostSimCtx,
//     ) -> substrate::error::Result<Self::Output> {
//         let data = ctx.output().data[0].tran();
//         let sig = SharedWaveform::from_signal(&data.data["vmeas"], &data.time);
//         let transitions = sig.transitions(
//             self.params.lower_threshold * self.params.vdd,
//             self.params.upper_threshold * self.params.vdd,
//         );
//
//         let t1 = data.time.values[idx1];
//         let t2 = data.time.values[idx2];
//
//         assert!(v2 > v1);
//         assert!(idx2 > idx1);
//         assert!(t2 > t1);
//
//         let cnode = self.params.idc as f64 * 1e-9 * (t2 - t1) / (v2 - v1);
//
//         Ok(NodeCap { cnode })
//     }
// }
