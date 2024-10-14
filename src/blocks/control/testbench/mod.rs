use std::path::PathBuf;
use std::sync::Arc;

use substrate::component::Component;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::capacitor::Capacitor;
use substrate::schematic::elements::mos::SchematicMos;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpwl::Vpwl;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::{Save, TranAnalysis};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::{TimeWaveform, Waveform};

use super::{ControlLogicParams, ControlLogicReplicaV2, InvChain};

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct TbParams {
    /// Clock period in seconds.
    pub clk_period: f64,
    /// Operations to test.
    #[builder(default, setter(into))]
    pub ops: Vec<Op>,
    /// Rise time of clock and inputs.
    pub tr: f64,
    /// Fall time of clock and inputs.
    pub tf: f64,
    /// Supply voltage.
    pub vdd: f64,
    /// Capacitance on output pins.
    pub c_load: f64,
    /// Hold time in seconds.
    ///
    /// Specifies how long data should be held after the clock edge.
    pub t_hold: f64,

    #[builder(default)]
    pub pex_netlist: Option<PathBuf>,
}

impl TbParams {
    #[inline]
    pub fn builder() -> TbParamsBuilder {
        TbParamsBuilder::default()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Read,
    Write,
    None,
    Reset,
}

pub struct ControlLogicTestbench {
    params: TbParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TbWaveforms {
    /// Clock.
    clk: Waveform,

    /// Chip enable.
    ce: Waveform,

    /// Write enable.
    we: Waveform,

    /// Reset.
    reset: Waveform,
}

fn generate_waveforms(params: &TbParams) -> TbWaveforms {
    let mut clk = Waveform::with_initial_value(0f64);
    let mut we = Waveform::with_initial_value(0f64);
    let mut ce = Waveform::with_initial_value(0f64);
    let mut reset = Waveform::with_initial_value(0f64);

    let period = params.clk_period;
    let vdd = params.vdd;
    let tr = params.tr;
    let tf = params.tf;

    let mut t = 0f64;
    let mut t_end;

    for op in params.ops.iter() {
        t_end = t + period;
        let t_data = t_end + params.t_hold;
        // Toggle the clock
        clk.push_high(t + (period / 2.0), vdd, tr);
        clk.push_low(t + period, vdd, tf);

        match op {
            Op::Read => {
                // Set write enable low
                we.push_low(t_data, vdd, tf);
                // Set chip enable high
                ce.push_high(t_data, vdd, tr);
                // Set reset low
                reset.push_low(t_data, vdd, tf);
            }
            Op::Write => {
                // Set write enable high
                we.push_high(t_data, vdd, tr);
                // Set chip enable high
                ce.push_high(t_data, vdd, tr);
                // Set reset low
                reset.push_low(t_data, vdd, tf);
            }

            Op::None => {
                // Set write enable low
                we.push_low(t_data, vdd, tf);
                // Set chip enable low
                ce.push_low(t_data, vdd, tf);
                // Set reset low
                reset.push_low(t_data, vdd, tf);
            }

            Op::Reset => {
                // Set write enable low
                we.push_low(t_data, vdd, tf);
                // Set chip enable low
                ce.push_low(t_data, vdd, tf);
                // Set reset low
                reset.push_high(t_data, vdd, tr);
            }
        }

        t += period;
    }

    t_end = t + period;
    let t_final = t + 2.0 * period + params.t_hold;

    // One more clock cycle
    clk.push_high(t + period / 2.0, vdd, tr);
    clk.push_low(t_end, vdd, tf);

    // Turn off control signals
    we.push_low(t_final, vdd, tf);
    ce.push_low(t_final, vdd, tf);
    reset.push_low(t_final, vdd, tf);
    clk.push_high(t_final, vdd, tr);

    TbWaveforms { clk, we, ce, reset }
}

impl Component for ControlLogicTestbench {
    type Params = TbParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("control_logic_testbench")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let [vdd, clk, we, ce, reset, saen, pc_b, wlen, wrdrven, rbl, decrepstart, decrepend] = ctx
            .signals([
                "vdd",
                "clk",
                "we",
                "ce",
                "reset",
                "saen",
                "pc_b",
                "wlen",
                "wrdrven",
                "rbl",
                "decrepstart",
                "decrepend",
            ]);

        let waveforms = generate_waveforms(&self.params);
        let output_cap = SiValue::with_precision(self.params.c_load, SiPrefix::Femto);

        ctx.instantiate::<ControlLogicReplicaV2>(&ControlLogicParams {
            decoder_delay_invs: 20,
            write_driver_delay_invs: 11,
        })?
        .with_connections([
            ("vdd", vdd),
            ("vss", vss),
            ("clk", clk),
            ("we", we),
            ("ce", ce),
            ("reset", reset),
            ("saen", saen),
            ("pc_b", pc_b),
            ("wlen", wlen),
            ("wrdrven", wrdrven),
            ("decrepstart", decrepstart),
            ("decrepend", decrepend),
            ("rbl", rbl),
        ])
        .named("dut")
        .add_to(ctx);

        ctx.instantiate::<InvChain>(&8)?
            .with_connections([
                ("din", decrepstart),
                ("dout", decrepend),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("decoder_replica")
            .add_to(ctx);

        ctx.instantiate::<Vdc>(&SiValue::with_precision(self.params.vdd, SiPrefix::Milli))?
            .with_connections([("p", vdd), ("n", vss)])
            .named("Vdd")
            .add_to(ctx);

        ctx.instantiate::<Vpwl>(&Arc::new(waveforms.clk))?
            .with_connections([("p", clk), ("n", vss)])
            .named("Vclk")
            .add_to(ctx);
        ctx.instantiate::<Vpwl>(&Arc::new(waveforms.we))?
            .with_connections([("p", we), ("n", vss)])
            .named("Vwe")
            .add_to(ctx);
        ctx.instantiate::<Vpwl>(&Arc::new(waveforms.ce))?
            .with_connections([("p", ce), ("n", vss)])
            .named("Vce")
            .add_to(ctx);
        ctx.instantiate::<Vpwl>(&Arc::new(waveforms.reset))?
            .with_connections([("p", reset), ("n", vss)])
            .named("Vreset")
            .add_to(ctx);
        ctx.instantiate::<Capacitor>(&output_cap)?
            .with_connections([("p", rbl), ("n", vss)])
            .named("Crbl")
            .add_to(ctx);

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();
        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();
        ctx.instantiate::<SchematicMos>(&MosParams {
            w: 1000,
            l: 150,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?
        .with_connections([("d", rbl), ("g", wlen), ("s", vss), ("b", vss)])
        .named("Mpd")
        .add_to(ctx);
        ctx.instantiate::<SchematicMos>(&MosParams {
            w: 1000,
            l: 150,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?
        .with_connections([("d", rbl), ("g", pc_b), ("s", vdd), ("b", vdd)])
        .named("Mpu")
        .add_to(ctx);

        Ok(())
    }
}

pub fn tb_params(vdd: f64) -> TbParams {
    let ops = vec![
        Op::Reset,
        Op::Read,
        Op::Write,
        Op::None,
        Op::Reset,
        Op::Write,
        Op::Read,
    ];

    let mut tb = TbParams::builder();
    let tb = tb
        .ops(ops)
        .clk_period(10.0e-9)
        .tr(40e-12)
        .tf(40e-12)
        .vdd(vdd)
        .c_load(2e-13)
        .t_hold(300e-12)
        .build()
        .unwrap();

    tb
}

impl Testbench for ControlLogicTestbench {
    type Output = ();
    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        let wav = generate_waveforms(&self.params);
        let step = self.params.clk_period / 8.0;
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        ctx.add_analysis(
            TranAnalysis::builder()
                .stop(wav.clk.last_t().unwrap() + 2.0 * step)
                // .stop(80e-9)
                .step(step)
                // .strobe_period(step)
                .build()
                .unwrap(),
        );

        ctx.save(Save::All);

        Ok(())
    }

    fn measure(
        &mut self,
        _ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        Ok(())
    }
}
