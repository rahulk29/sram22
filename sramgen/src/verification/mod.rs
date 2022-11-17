use std::path::PathBuf;

use bit_signal::BitSignal;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use waveform::Waveform;

use crate::verification::utils::push_bus;
use crate::{Result, BUILD_PATH, LIB_PATH};

use self::netlist::{generate_netlist, write_netlist, TbNetlistParams};

pub mod bit_signal;
pub mod netlist;
pub mod spectre;
pub mod utils;
pub mod waveform;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PortOrder {
    MsbFirst,
    LsbFirst,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PortClass {
    Addr,
    DataIn,
    DataOut,
    Power,
    Clock,
    Ground,
    WriteMask,
    WriteEnable,
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
pub struct TestCase {
    pub clk_period: f64,
    pub ops: Vec<Op>,
}

impl TestCase {
    #[inline]
    pub fn builder() -> TestCaseBuilder {
        TestCaseBuilder::default()
    }
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct TbParams {
    pub test_case: TestCase,
    /// Name of the SRAM subcircuit.
    #[builder(setter(into))]
    pub sram_name: String,
    /// Rise time of clock and inputs.
    pub tr: f64,
    /// Fall time of clock and inputs.
    pub tf: f64,
    /// Supply voltage.
    pub vdd: f64,
    /// Capacitance on output pins.
    pub c_load: f64,

    /// Number of data bits.
    pub data_width: usize,
    /// Number of address bits.
    pub addr_width: usize,
    /// Number of write mask bits.
    pub wmask_groups: usize,

    /// Ports in the order in which they appear in the SRAM
    /// SPICE subcircuit definition.
    ///
    /// For single bit ports, the [`PortOrder`] is ignored.
    #[builder(setter(into))]
    pub ports: Vec<(PortClass, PortOrder)>,
    /// Name of the clock port
    #[builder(setter(into))]
    pub clk_port: String,
    /// Name of the write enable port.
    #[builder(setter(into))]
    pub write_enable_port: String,
    /// Name of the address bus.
    #[builder(setter(into))]
    pub addr_port: String,
    /// Name of the data input bus.
    #[builder(setter(into))]
    pub data_in_port: String,
    /// Name of the data output bus.
    #[builder(setter(into))]
    pub data_out_port: String,
    /// Name of the power supply port (VPWR/VDD).
    #[builder(setter(into))]
    pub pwr_port: String,
    /// Name of the ground port (VGND/VSS).
    #[builder(setter(into))]
    pub gnd_port: String,
    /// Name of the write mask bus.
    #[builder(default, setter(strip_option, into))]
    pub wmask_port: Option<String>,

    /// Working directory for the simulator and generated files.
    #[builder(setter(into))]
    pub work_dir: PathBuf,
    /// Source netlists.
    #[builder(default, setter(into))]
    pub source_paths: Vec<PathBuf>,
    /// Additional SPICE files to include.
    ///
    /// Should NOT include the source paths
    /// specified in [`TbParams::source_paths`].
    #[builder(default, setter(into))]
    pub includes: Vec<String>,
}

impl TbParams {
    pub fn port_name(&self, port_class: PortClass) -> &str {
        match port_class {
            PortClass::Addr => &self.addr_port,
            PortClass::DataIn => &self.data_in_port,
            PortClass::DataOut => &self.data_out_port,
            PortClass::Ground => &self.gnd_port,
            PortClass::Power => &self.pwr_port,
            PortClass::Clock => &self.clk_port,
            PortClass::WriteMask => self.wmask_port.as_ref().unwrap(),
            PortClass::WriteEnable => &self.write_enable_port,
        }
    }
    pub fn port_width(&self, port_class: PortClass) -> usize {
        match port_class {
            PortClass::Addr => self.addr_width,
            PortClass::DataIn => self.data_width,
            PortClass::DataOut => self.data_width,
            PortClass::Ground => 1,
            PortClass::Power => 1,
            PortClass::Clock => 1,
            PortClass::WriteMask => self.wmask_groups,
            PortClass::WriteEnable => 1,
        }
    }
    #[inline]
    pub fn builder() -> TbParamsBuilder {
        TbParamsBuilder::default()
    }
}

impl PortClass {
    pub fn is_bus(&self) -> bool {
        match *self {
            PortClass::Addr => true,
            PortClass::DataIn => true,
            PortClass::DataOut => true,
            PortClass::Ground => false,
            PortClass::Power => false,
            PortClass::Clock => false,
            PortClass::WriteMask => true,
            PortClass::WriteEnable => false,
        }
    }
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

pub fn run_testbench(params: &TbParams) -> Result<()> {
    let waveforms = generate_waveforms(params);
    let netlist = generate_netlist(TbNetlistParams {
        tb: params,
        waveforms: &waveforms,
    })?;

    write_netlist(
        params
            .work_dir
            .join(format!("test_{}_sim.sp", params.sram_name)),
        &netlist,
    )?;
    // write_netlist
    // run_simulation
    // parse_results
    // verify_results

    Ok(())
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
    let mut t_end;

    for op in params.test_case.ops.iter() {
        t_end = t + period;
        // Toggle the clock
        clk.push_high(t + (period / 2.0), vdd, tr);
        clk.push_low(t + period, vdd, tf);

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

    t_end = t + period;
    let t_final = t + 2.0 * period;

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

pub fn source_files(sram_name: &str) -> Vec<PathBuf> {
        let source_path_main = PathBuf::from(BUILD_PATH).join(format!("spice/{}.spice", sram_name));
        let source_path_dff = PathBuf::from(LIB_PATH).join("openram_dff/openram_dff.spice");
        let source_path_sp_cell =
            PathBuf::from(LIB_PATH).join("sram_sp_cell/sky130_fd_bd_sram__sram_sp_cell.lvs.spice");
        let source_path_sp_sense_amp =
            PathBuf::from(LIB_PATH).join("sramgen_sp_sense_amp/sramgen_sp_sense_amp.spice");
        let source_path_control_simple =
            PathBuf::from(LIB_PATH).join("sramgen_control/sramgen_control_simple.spice");

        vec![
            source_path_main,
            source_path_dff,
            source_path_sp_cell,
            source_path_sp_sense_amp,
            source_path_control_simple,
        ]
}
