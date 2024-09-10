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
use substrate::schematic::signal::Signal;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::{Analysis, Save, TranAnalysis};

use crate::pex::Pex;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum TbNode {
    Vdd,
    Vss,
    // Node to be measured.
    Vmeas,
    Floating,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeCap {
    pub cnode: f64,
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct TbParams<T> {
    /// Current source value in nano amperes.
    pub idc: i64,
    /// Supply voltage.
    pub vdd: f64,
    pub dut: T,
    pub pex_netlist: Option<PathBuf>,
    pub connections: HashMap<ArcStr, Vec<TbNode>>,
}

impl<T: Clone> TbParams<T> {
    #[inline]
    pub fn builder() -> TbParamsBuilder<T> {
        TbParamsBuilder::default()
    }
}

pub struct CapTestbench<T: Component> {
    params: TbParams<T::Params>,
}

impl<P: Clone + Serialize, T: Component<Params = P>> Component for CapTestbench<T> {
    type Params = TbParams<T::Params>;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("cap_testbench")
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
                    TbNode::Vdd => vdd,
                    TbNode::Vss => vss,
                    TbNode::Vmeas => vmeas,
                    TbNode::Floating => {
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

        let mut idc = ctx.instantiate::<Idc>(&SiValue::new(self.params.idc, SiPrefix::Nano))?;
        idc.connect_all([("p", vss), ("n", vmeas)]);
        idc.set_name("iin");
        ctx.add_instance(idc);

        ctx.set_spice(".ic v(vmeas)=0");

        Ok(())
    }
}

impl<P: Clone + Serialize, T: Component<Params = P>> Testbench for CapTestbench<T> {
    type Output = NodeCap;

    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        ctx.add_analysis(Analysis::Tran(
            TranAnalysis::builder()
                .stop(6e-6)
                .start(0.0)
                .step(1e-9)
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
        let sig = &data.data["vmeas"];
        let (idx1, v1) = sig
            .values
            .iter()
            .enumerate()
            .find(|(_i, &x)| x > 0.1)
            .unwrap();
        let (idx2, v2) = sig
            .values
            .iter()
            .enumerate()
            .find(|(_i, &x)| x > 0.5)
            .unwrap();

        let t1 = data.time.values[idx1];
        let t2 = data.time.values[idx2];

        assert!(v2 > v1);
        assert!(idx2 > idx1);
        assert!(t2 > t1);

        let cnode = self.params.idc as f64 * 1e-9 * (t2 - t1) / (v2 - v1);

        Ok(NodeCap { cnode })
    }
}
