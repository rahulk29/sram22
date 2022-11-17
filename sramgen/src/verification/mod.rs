use bit_signal::BitSignal;
use waveform::Waveform;

use crate::verification::utils::push_bus;

pub mod bit_signal;
pub mod utils;
pub mod waveform;

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

pub struct TestCase {
    pub clk_period: f64,
    pub ops: Vec<Op>,
}

pub struct TbParams {
    pub test_case: TestCase,

    /// Name of the SRAM subcircuit.
    pub sram_name: String,

    /// Rise time.
    pub tr: f64,
    /// Fall time.
    pub tf: f64,

    /// Supply voltage.
    pub vdd: f64,

    pub data_width: usize,
    pub addr_width: usize,
    pub wmask_groups: usize,

    pub chip_select_port: Option<String>,
    pub write_enable_port: String,
    pub addr_port: String,
    pub data_in_port: String,
    pub data_out_port: String,
}

struct TbWaveforms {
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

pub fn run_testbench(params: &TbParams) {
    let waveforms = generate_waveforms(params);
}

fn generate_waveforms(params: &TbParams) -> TbWaveforms {
    let mut addr = vec![Waveform::with_initial_value(0f64); params.addr_width];
    let mut din = vec![Waveform::with_initial_value(0f64); params.data_width];
    let wmask_bits = if params.wmask_groups > 1 {
        params.wmask_groups
    } else {
        0
    };
    let mut wmask = vec![Waveform::with_initial_value(0f64); wmask_bits];
    let mut clk = Waveform::with_initial_value(0f64);
    let mut we = Waveform::with_initial_value(0f64);

    let period = params.test_case.clk_period;
    let vdd = params.vdd;
    let tr = params.tr;
    let tf = params.tf;

    let mut t = 0f64;

    for op in params.test_case.ops.iter() {
        let t_end = t + period;
        // Toggle the clock
        clk.push_high(t + period / 2.0, vdd, tr);
        clk.push_low(t_end, vdd, tf);

        match op {
            Op::Read { addr: addrv } => {
                // Set write enable low
                we.push_low(t_end, vdd, tf);

                assert_eq!(addrv.width(), params.addr_width);
                push_bus(&mut addr, addrv, t_end, vdd, tr, tf);
            }
            Op::Write { addr: addrv, data } => {
                // Set write enable high
                we.push_high(t_end, vdd, tr);

                assert_eq!(addrv.width(), params.addr_width);
                push_bus(&mut addr, addrv, t_end, vdd, tr, tf);

                assert_eq!(data.width(), params.data_width);
                push_bus(&mut din, data, t_end, vdd, tr, tf);
            }

            Op::WriteMasked {
                addr: addrv,
                data,
                mask,
            } => {
                // Set write enable high
                we.push_high(t_end, vdd, tr);

                assert_eq!(addrv.width(), params.addr_width);
                push_bus(&mut addr, addrv, t_end, vdd, tr, tf);

                assert_eq!(data.width(), params.data_width);
                push_bus(&mut din, data, t_end, vdd, tr, tf);

                assert!(params.wmask_groups > 1);
                assert_eq!(mask.width(), params.wmask_groups);
                push_bus(&mut wmask, mask, t_end, vdd, tr, tf);
            }
        }

        t += period;
    }

    TbWaveforms {
        addr,
        din,
        clk,
        we,
        wmask,
    }
}
