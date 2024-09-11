use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::PathBuf;

use arcstr::ArcStr;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use substrate::component::Component;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::iac::Iac;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpulse::Vpulse;
use substrate::schematic::signal::Signal;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::{EdgeDir, SharedWaveform, TimeWaveform};
use substrate::verification::simulation::{
    AcAnalysis, Analysis, ComplexSignal, RealSignal, Save, SweepMode, TranAnalysis,
};

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

impl<P: Clone + Serialize, T: Component<Params = P>> Testbench for TransitionTestbench<T> {
    type Output = TransitionTimes;

    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        let duration = (self.params.delay + self.params.width) * 2.;
        ctx.add_analysis(Analysis::Tran(
            TranAnalysis::builder()
                .stop(duration)
                .start(0.0)
                .step(duration / 1e5)
                .build()
                .unwrap(),
        ))
        .save(Save::All);
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        let data = ctx.output().data[0].tran();
        let sig = SharedWaveform::from_signal(&data.time, &data.data["vmeas"]);
        let transitions = sig
            .transitions(
                self.params.lower_threshold * self.params.vdd,
                self.params.upper_threshold * self.params.vdd,
            )
            .collect::<Vec<_>>();

        let t1 = transitions[0];
        let t2 = transitions[1];

        let (tr, tf) = match t1.dir() {
            EdgeDir::Rising => (t1.duration(), t2.duration()),
            EdgeDir::Falling => (t2.duration(), t1.duration()),
        };

        Ok(TransitionTimes { tr, tf })
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AcImpedanceTbNode {
    Vdd,
    Vss,
    Vcustom(SiValue),
    // Node to be measured.
    Vmeas,
    Floating,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcImpedance {
    pub vmeas: ComplexSignal,
    pub freq: RealSignal,
}

impl AcImpedance {
    /// Returns the capacitance at high frequency from the reactance.
    pub fn max_freq_cap(&self) -> f64 {
        -1. / self.vmeas.imag.last().unwrap() / self.freq.values.last().unwrap() / 2. / PI
    }

    /// Returns the resistance at low frequency.
    pub fn min_freq_res(&self) -> f64 {
        *self.vmeas.real.first().unwrap()
    }
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct AcImpedanceTbParams<T> {
    /// Supply voltage.
    pub vdd: f64,
    pub fstart: f64,
    pub fstop: f64,
    pub points: usize,
    pub dut: T,
    pub pex_netlist: Option<PathBuf>,
    pub connections: HashMap<ArcStr, Vec<AcImpedanceTbNode>>,
}

impl<T: Clone> AcImpedanceTbParams<T> {
    #[inline]
    pub fn builder() -> AcImpedanceTbParamsBuilder<T> {
        AcImpedanceTbParamsBuilder::default()
    }
}

pub struct AcImpedanceTestbench<T: Component> {
    params: AcImpedanceTbParams<T::Params>,
}

impl<P: Clone + Serialize, T: Component<Params = P>> Component for AcImpedanceTestbench<T> {
    type Params = AcImpedanceTbParams<T::Params>;
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
        let mut ctr = 0;

        let mut connections = Vec::new();

        for (k, nodes) in &self.params.connections {
            let mut signal = Vec::new();
            for node in nodes {
                signal.push(match node {
                    AcImpedanceTbNode::Vdd => vdd,
                    AcImpedanceTbNode::Vss => vss,
                    AcImpedanceTbNode::Vmeas => vmeas,
                    AcImpedanceTbNode::Vcustom(val) => {
                        let vcustom = ctx.signal(format!("custom{ctr}"));
                        ctx.instantiate::<Vdc>(val)?
                            .with_connections([("p", vcustom), ("n", vss)])
                            .named(format!("Vcustom{ctr}"))
                            .add_to(ctx);
                        vcustom
                    }
                    AcImpedanceTbNode::Floating => {
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

        ctx.instantiate::<Iac>(&SiValue::with_precision(1., SiPrefix::None))?
            .with_connections([("p", vss), ("n", vmeas)])
            .named("iin")
            .add_to(ctx);

        Ok(())
    }
}

impl<P: Clone + Serialize, T: Component<Params = P>> Testbench for AcImpedanceTestbench<T> {
    type Output = AcImpedance;

    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        ctx.add_analysis(Analysis::Ac(
            AcAnalysis::builder()
                .fstop(self.params.fstop)
                .fstart(self.params.fstart)
                .points(self.params.points)
                .sweep(SweepMode::Lin)
                .build()
                .unwrap(),
        ))
        .save(Save::Signals(["vmeas".into()].into_iter().collect()));
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        let data = ctx.output().data[0].ac();

        Ok(AcImpedance {
            vmeas: data.data["vmeas"].clone(),
            freq: data.freq.clone(),
        })
    }
}
