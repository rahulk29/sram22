use std::path::PathBuf;
use std::sync::Arc;

use substrate::component::Component;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::capacitor::Capacitor;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpwl::Vpwl;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::bits::BitSignal;
use substrate::verification::simulation::{Save, TranAnalysis};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::{TimeWaveform, Waveform};

use super::{Sram, SramParams, SramPex, SramPexParams};

pub mod verify;

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

    /// SRAM configuration to test.
    pub sram: SramParams,
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
    Read {
        addr: BitSignal,
    },
    Write {
        addr: BitSignal,
        data: BitSignal,
    },
    WriteMasked {
        addr: BitSignal,
        data: BitSignal,
        mask: BitSignal,
    },
}

pub struct SramTestbench {
    params: TbParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TbWaveforms {
    /// One [`Waveform`] per address bit.
    addr: Vec<Waveform>,

    /// One [`Waveform`] per data bit.
    din: Vec<Waveform>,

    /// Clock.
    clk: Waveform,

    /// Write enable.
    we: Waveform,

    /// One [`Waveform`] per write mask bit.
    ///
    /// Empty if no write mask is enabled.
    wmask: Vec<Waveform>,
}

pub fn push_bus(
    waveforms: &mut [Waveform],
    signal: &BitSignal,
    until: f64,
    vdd: f64,
    tr: f64,
    tf: f64,
) {
    assert_eq!(waveforms.len(), signal.width());
    for (i, bit) in signal.bits().enumerate() {
        if bit {
            waveforms[i].push_high(until, vdd, tr);
        } else {
            waveforms[i].push_low(until, vdd, tf);
        }
    }
}

fn generate_waveforms(params: &TbParams) -> TbWaveforms {
    let mut addr = vec![Waveform::with_initial_value(0f64); params.sram.addr_width];
    let mut din = vec![Waveform::with_initial_value(0f64); params.sram.data_width];
    let wmask_bits = if params.sram.wmask_width > 1 {
        params.sram.wmask_width
    } else {
        0
    };
    let mut wmask = vec![Waveform::with_initial_value(0f64); wmask_bits];
    let mut clk = Waveform::with_initial_value(0f64);
    let mut we = Waveform::with_initial_value(0f64);

    let period = params.clk_period;
    let vdd = params.vdd;
    let tr = params.tr;
    let tf = params.tf;

    let mut t = 0f64;
    let mut t_end;

    let wmask_all = BitSignal::ones(params.sram.wmask_width);

    for op in params.ops.iter() {
        t_end = t + period;
        let t_data = t_end + params.t_hold;
        // Toggle the clock
        clk.push_high(t + (period / 2.0), vdd, tr);
        clk.push_low(t + period, vdd, tf);

        match op {
            Op::Read { addr: addrv } => {
                // Set write enable low
                we.push_low(t_data, vdd, tf);

                assert_eq!(addrv.width(), params.sram.addr_width);
                push_bus(&mut addr, addrv, t_data, vdd, tr, tf);
            }
            Op::Write { addr: addrv, data } => {
                // Set write enable high
                we.push_high(t_data, vdd, tr);

                assert_eq!(addrv.width(), params.sram.addr_width);
                push_bus(&mut addr, addrv, t_data, vdd, tr, tf);

                assert_eq!(data.width(), params.sram.data_width);
                push_bus(&mut din, data, t_data, vdd, tr, tf);

                if params.sram.wmask_width > 1 {
                    push_bus(&mut wmask, &wmask_all, t_data, vdd, tr, tf);
                }
            }

            Op::WriteMasked {
                addr: addrv,
                data,
                mask,
            } => {
                // Set write enable high
                we.push_high(t_data, vdd, tr);

                assert_eq!(addrv.width(), params.sram.addr_width);
                push_bus(&mut addr, addrv, t_data, vdd, tr, tf);

                assert_eq!(data.width(), params.sram.data_width);
                push_bus(&mut din, data, t_data, vdd, tr, tf);

                assert!(params.sram.wmask_width > 1);
                assert_eq!(mask.width(), params.sram.wmask_width);
                push_bus(&mut wmask, mask, t_data, vdd, tr, tf);
            }
        }

        t += period;
    }

    t_end = t + period;
    let t_final = t + 2.0 * period + params.t_hold;

    // One more clock cycle
    clk.push_high(t + period / 2.0, vdd, tr);
    clk.push_low(t_end, vdd, tf);

    // Turn off write enable
    we.push_low(t_final, vdd, tf);
    clk.push_high(t_final, vdd, tr);

    TbWaveforms {
        addr,
        din,
        clk,
        we,
        wmask,
    }
}

impl Component for SramTestbench {
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
        arcstr::literal!("sram_testbench")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let vdd = ctx.signal("vdd");

        let clk = ctx.signal("clk");
        let we = ctx.signal("we");
        let addr = ctx.bus("addr", self.params.sram.addr_width);
        let din = ctx.bus("din", self.params.sram.data_width);
        let dout = ctx.bus("dout", self.params.sram.data_width);
        let wmask = ctx.bus("wmask", self.params.sram.wmask_width);

        let waveforms = generate_waveforms(&self.params);
        let output_cap = SiValue::with_precision(self.params.c_load, SiPrefix::Femto);

        if let Some(ref pex_netlist) = self.params.pex_netlist {
            ctx.instantiate::<SramPex>(&SramPexParams {
                params: self.params.sram.clone(),
                pex_netlist: pex_netlist.clone(),
            })?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("clk", clk),
                ("we", we),
                ("addr", addr),
                ("wmask", wmask),
                ("din", din),
                ("dout", dout),
            ])
            .named("dut")
            .add_to(ctx);
        } else {
            ctx.instantiate::<Sram>(&self.params.sram)?
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("clk", clk),
                    ("we", we),
                    ("addr", addr),
                    ("wmask", wmask),
                    ("din", din),
                    ("dout", dout),
                ])
                .named("dut")
                .add_to(ctx);
        }

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
        for i in 0..self.params.sram.addr_width {
            ctx.instantiate::<Vpwl>(&Arc::new(waveforms.addr[i].clone()))?
                .with_connections([("p", addr.index(i)), ("n", vss)])
                .named(format!("Vaddr_{i}"))
                .add_to(ctx);
        }
        for i in 0..self.params.sram.wmask_width {
            ctx.instantiate::<Vpwl>(&Arc::new(waveforms.wmask[i].clone()))?
                .with_connections([("p", wmask.index(i)), ("n", vss)])
                .named(format!("Vwmask_{i}"))
                .add_to(ctx);
        }
        for i in 0..self.params.sram.data_width {
            ctx.instantiate::<Vpwl>(&Arc::new(waveforms.din[i].clone()))?
                .with_connections([("p", din.index(i)), ("n", vss)])
                .named(format!("Vdin_{i}"))
                .add_to(ctx);
            ctx.instantiate::<Capacitor>(&output_cap)?
                .with_connections([("p", dout.index(i)), ("n", vss)])
                .named(format!("Co_{i}"))
                .add_to(ctx);
        }

        Ok(())
    }
}

pub fn tb_params(
    params: SramParams,
    vdd: f64,
    short: bool,
    pex_netlist: Option<PathBuf>,
) -> TbParams {
    let wmask_width = params.wmask_width;
    let data_width = params.data_width;
    let addr_width = params.addr_width;

    // An alternating 64-bit sequence 0b010101...01
    let bit_pattern1 = 0x5555555555555555u64;

    // An alternating 64-bit sequence 0b101010...10
    let bit_pattern2 = 0xAAAAAAAAAAAAAAAAu64;

    let addr1 = BitSignal::zeros(addr_width);
    let addr2 = BitSignal::ones(addr_width);

    let mut ops = vec![
        Op::Write {
            addr: addr1.clone(),
            data: BitSignal::from_u64(bit_pattern1, data_width),
        },
        Op::Write {
            addr: addr2.clone(),
            data: BitSignal::from_u64(bit_pattern2, data_width),
        },
        Op::Read {
            addr: addr1.clone(),
        },
        Op::Read { addr: addr2 },
        Op::Read { addr: addr1 },
    ];

    if !short {
        for i in 0..16 {
            let bits = (i % 2) * bit_pattern2 + (1 - (i % 2)) * bit_pattern1 + i + 1;
            ops.push(Op::Write {
                addr: BitSignal::from_u64(i, addr_width),
                data: BitSignal::from_u64(bits, data_width),
            });
        }
        for i in 0..16 {
            ops.push(Op::Read {
                addr: BitSignal::from_u64(i, addr_width),
            });
        }

        if wmask_width > 1 {
            for i in 0..16 {
                let bits = (1 - (i % 2)) * bit_pattern2 + (i % 2) * bit_pattern1 + i + 1;
                ops.push(Op::WriteMasked {
                    addr: BitSignal::from_u64(i, addr_width),
                    data: BitSignal::from_u64(bits, data_width),
                    mask: BitSignal::from_u64(bit_pattern1 + i * 0b10110010111, wmask_width),
                });
            }
            for i in 0..16 {
                ops.push(Op::Read {
                    addr: BitSignal::from_u64(i, addr_width),
                });
            }
        }
    }

    let mut tb = TbParams::builder();
    let tb = tb
        .ops(ops)
        .clk_period(10.0e-9)
        .tr(40e-12)
        .tf(40e-12)
        .vdd(vdd)
        .c_load(5e-15)
        .t_hold(300e-12)
        .sram(params)
        .pex_netlist(pex_netlist)
        .build()
        .unwrap();

    tb
}

impl Testbench for SramTestbench {
    type Output = ();
    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        let wav = generate_waveforms(&self.params);
        let step = self.params.clk_period / 8.0;
        use std::collections::HashMap;
        let opts = HashMap::from_iter([
            ("write".to_string(), "initial.ic".to_string()),
            ("readns".to_string(), "initial.ic".to_string()),
        ]);
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        ctx.add_analysis(
            TranAnalysis::builder()
                .stop(wav.clk.last_t().unwrap())
                // .stop(80e-9)
                .step(step)
                .strobe_period(step)
                .opts(opts)
                .build()
                .unwrap(),
        );

        let signals = (0..self.params.sram.data_width)
            .map(|i| format!("dout[{i}]"))
            .collect();
        ctx.save(Save::Signals(signals));
        // ctx.save(Save::All);

        let vdd = SiValue::with_precision(self.params.vdd, SiPrefix::Nano);

        for i in 0..self.params.sram.rows {
            ctx.set_ic(format!("Xdut.X0.wl[{i}]"), SiValue::zero());
            for j in 0..self.params.sram.cols {
                ctx.set_ic(
                    format!("Xdut.X0.Xbitcell_array.Xcell_{i}_{j}.X0.Q"),
                    SiValue::zero(),
                );
                ctx.set_ic(format!("Xdut.X0.Xbitcell_array.Xcell_{i}_{j}.X0.QB"), vdd);
            }
        }
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        let data = ctx.output().data[0].tran();
        verify::verify_simulation(data, &self.params)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use std::env;
    use substrate::schematic::netlist::NetlistPurpose;

    fn test_sram(name: &str, params: SramParams) {
        env::set_var(
            "SPECTRE_FLAGS",
            "+preset=mx +postlpreset=mx +logstatus +mt=8 -64 +error +warn +note",
        );
        let ctx = setup_ctx();
        let corners = ctx.corner_db();

        let short = false;
        let short_str = if short { "short" } else { "long" };

        let work_dir = test_work_dir(name);

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<Sram>(&params, &gds_path)
            .expect("failed to write layout");

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<Sram>(&params, &pex_path, NetlistPurpose::Pex)
            .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: params.name().clone(),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: params.name().clone(),
            pex_netlist_path: pex_netlist_path.clone(),
            opts,
        })
        .expect("failed to run pex");

        let mut handles = Vec::new();
        for vdd in [1.8, 1.5, 2.0] {
            for corner in corners.corners() {
                let corner = corner.clone();
                let params = params.clone();
                let name = name.to_string();
                let pex_netlist = Some(pex_netlist_path.clone());
                handles.push(std::thread::spawn(move || {
                    let ctx = setup_ctx();
                    let tb = tb_params(params, vdd, short, pex_netlist);
                    let work_dir = test_work_dir(&format!(
                        "{}/{}_{:.2}_{}",
                        name,
                        corner.name(),
                        vdd,
                        short_str
                    ));
                    ctx.write_simulation_with_corner::<SramTestbench>(
                        &tb,
                        &work_dir,
                        corner.clone(),
                    )
                    .expect("failed to run simulation");
                    println!(
                        "Simulated corner {} with Vdd = {}, short = {}",
                        corner.name(),
                        vdd,
                        short
                    );
                }));
            }
        }

        for h in handles {
            h.join().expect("failed to join on thread");
        }
    }

    macro_rules! test_tb_sram {
        ($name: ident, $params: ident) => {
            #[test]
            #[ignore = "slow"]
            fn $name() {
                test_sram(stringify!($name), $params);
            }
        };
    }

    test_tb_sram!(test_tb_sram22_64x4m4w2, SRAM22_64X4M4W2);
    test_tb_sram!(test_tb_sram22_64x24m4w24, SRAM22_64X24M4W24);
    test_tb_sram!(test_tb_sram22_64x32m4w8, SRAM22_64X32M4W8);
    test_tb_sram!(test_tb_sram22_64x32m4w32, SRAM22_64X32M4W32);
    test_tb_sram!(test_tb_sram22_256x32m4w8, SRAM22_256X32M4W8);
    test_tb_sram!(test_tb_sram22_512x32m4w8, SRAM22_512X32M4W8);
    test_tb_sram!(test_tb_sram22_512x32m4w32, SRAM22_512X32M4W32);
    test_tb_sram!(test_tb_sram22_512x64m4w8, SRAM22_512X64M4W8);
    test_tb_sram!(test_tb_sram22_1024x32m8w8, SRAM22_1024X32M8W8);
    test_tb_sram!(test_tb_sram22_1024x32m8w32, SRAM22_1024X32M8W32);
    test_tb_sram!(test_tb_sram22_1024x64m8w32, SRAM22_1024X64M8W32);
    test_tb_sram!(test_tb_sram22_2048x32m8w8, SRAM22_2048X32M8W8);
    test_tb_sram!(test_tb_sram22_2048x64m4w8, SRAM22_2048X64M4W8);
    test_tb_sram!(test_tb_sram22_4096x8m8w8, SRAM22_4096X8M8W8);
    test_tb_sram!(test_tb_sram22_4096x32m8w8, SRAM22_4096X32M8W8);
}
