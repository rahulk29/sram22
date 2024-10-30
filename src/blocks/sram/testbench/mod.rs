use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

use calibre::pex::PexLevel;
use substrate::component::Component;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::capacitor::Capacitor;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpwl::Vpwl;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::bits::BitSignal;
use substrate::verification::simulation::{Save, TranAnalysis, TranData};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::{TimeWaveform, Waveform};

use super::{Sram, SramParams, SramPex, SramPexParams, SramPhysicalDesign};

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
    pub dsn: Arc<SramPhysicalDesign>,
    pub pex_netlist: Option<(PathBuf, PexLevel)>,
}

#[derive(Debug, Clone, Copy)]
pub enum TbSignals {
    Clk,
    We,
    Ce,
    RstB,
    Addr(usize),
    Wmask(usize),
    Din(usize),
    Dout(usize),
    Wlen,
    Wlen0,
    Decrepstart,
    Decrepend,
    PcBStart,
    PcBEnd,
    PcB0,
    SenseEnStart,
    SenseEnEnd,
    SenseEn0,
    Rwl,
    Rbl,
    WriteDriverEnStart,
    WriteDriverEnEnd,
    WriteDriverEn0,
    WlStart(usize),
    WlEnd(usize),
    WlBs,
    WeI(usize),
    WeIb(usize),
    Bl(usize),
    Br(usize),
    BlOut(usize),
    BrOut(usize),
    AddrGated(usize),
    AddrBGated(usize),
    ColSelStart(usize),
    ColSelEnd(usize),
    ColSelBStart(usize),
    ColSelBEnd(usize),
    BitcellQ(usize, usize),
    BitcellQB(usize, usize),
    WlCtlQ,
    WlCtlQB,
    SaenCtlQ,
    SaenCtlQB,
    PcCtlQ,
    PcCtlQB,
    WrdrvenCtlQ,
    WrdrvenCtlQB,
    DffsQ1(usize),
    DffsQ1B(usize),
    DffsQ2(usize),
    DffsQ2B(usize),
    ChildConnsStart,
    ChildConnsEnd,
    LastStageDecoderXs,
    LastStageDecoderVdds,
    LastStageDecoderVsss,
    ColumnVdds,
    ColumnVsss,
    WriteDriverQ(usize),
    WriteDriverQB(usize),
    WmaskQ(usize),
}

impl TbParams {
    #[inline]
    pub fn builder() -> TbParamsBuilder {
        TbParamsBuilder::default()
    }

    pub fn sram_signal_path(&self, signal: TbSignals) -> String {
        let mut last_stage_decoder_depth = 0;
        let mut node = &self.dsn.row_decoder.tree.root;
        let num_children = node.children.len();
        while num_children == 1 {
            if node.gate.gate_type().is_inv() {
                last_stage_decoder_depth += 1;
                node = &node.children[0];
            } else {
                break;
            }
        }

        match signal {
            TbSignals::Clk => "clk".to_string(),
            TbSignals::We => "we".to_string(),
            TbSignals::Ce => "ce".to_string(),
            TbSignals::RstB => "rstb".to_string(),
            TbSignals::Addr(i) => format!("addr[{i}]"),
            TbSignals::Wmask(i) => format!("wmask[{i}]"),
            TbSignals::Din(i) => format!("din[{i}]"),
            TbSignals::Dout(i) => format!("dout[{i}]"),
            _ => {
                if let Some((_, ref level)) = self.pex_netlist {
                    format!(
                        "Xdut.Xdut.{}",
                        match level {
                            PexLevel::Rc => {
                                match signal {
                                    TbSignals::Clk | TbSignals::We | TbSignals::Ce | TbSignals::RstB | TbSignals::Addr(_) | TbSignals::Wmask(_) | TbSignals::Din(_) | TbSignals::Dout(_) => unreachable!(),
                                    TbSignals::Wlen => format!("N_X0/wl_en_X0/Xaddr_gate/Xgate_0_{}_0/X0/Xn1/M0_g", self.sram.row_bits() - 1),
                                    TbSignals::Decrepstart => "N_X0/Xcontrol_logic/decrepstart_X0/Xcontrol_logic/Xmux_wlen_rst/X0/X21/M0_d".to_string(),
                                    TbSignals::Decrepend => "N_X0/Xcontrol_logic/decrepend_X0/Xcontrol_logic/Xdecoder_replica_delay/Xinv0/X0/X0/M0_g".to_string(),
                                    TbSignals::PcBStart => "N_X0/pc_b_X0/Xcol_circuitry/Xcol_group_0/Xprecharge_0/Xbl_pull_up/M0_g".to_string(),
                                    TbSignals::PcBEnd => format!("N_X0/pc_b_X0/Xcol_circuitry/Xcol_group_{}/Xprecharge_0/Xbl_pull_up/M0_g", self.sram.data_width() - 1),
                                    TbSignals::SenseEnStart => "N_X0/sense_en_X0/Xcol_circuitry/Xcol_group_0/Xsense_amp/X0/XSWOP/M0_g".to_string(),
                                    TbSignals::SenseEnEnd => format!("N_X0/sense_en_X0/Xcol_circuitry/Xcol_group_{}/Xsense_amp/X0/XSWOP/M0_g", self.sram.data_width() - 1),
                                    TbSignals::Rwl => "N_X0/rwl_X0/Xcontrol_logic/Xrwl_buf/X0/X41/M0_s".to_string(),
                                    TbSignals::Rbl => "N_X0/rbl_X0/Xcontrol_logic/Xinv_rbl/X0/X0/M0_g".to_string(),
                                    TbSignals::WriteDriverEnStart => "N_X0/write_driver_en_X0/Xcol_circuitry/Xwmask_and_0/Xgate_0_0_0/Xn1/M0_g".to_string(),
                                    TbSignals::WriteDriverEnEnd => format!("N_X0/write_driver_en_X0/Xcol_circuitry/Xwmask_and_{}/Xgate_0_0_0/Xn1/M0_g", self.sram.wmask_width() - 1),
                                    TbSignals::WlStart(i) => format!("N_X0/wl[{i}]_X0/Xdecoder/X0_2/Xgate_{last_stage_decoder_depth}_{i}_0/XMN1/M0_d"),
                                    TbSignals::WlEnd(i) => format!("N_X0/wl[{i}]_X0/Xbitcell_array/Xcell_{i}_{}/X0/X2/M0_g", self.sram.rows() - 1),
                                    TbSignals::WeI(i) => format!("N_X0/Xcol_circuitry/we_i{}_X0/Xcol_circuitry/Xcol_group_{}/Xwrite_driver/Xbrdriver/Xmn_en/M0_g",if self.sram.wmask_width() > 1 { format!("[{i}]") } else { "".to_string() }, (i + 1) * self.sram.wmask_granularity() - 1),
                                    TbSignals::WeIb(i) => format!("N_X0/Xcol_circuitry/we_ib{}_X0/Xcol_circuitry/Xcol_group_{}/Xwrite_driver/Xbrdriver/Xmp_en/M0_g",if self.sram.wmask_width() > 1 { format!("[{i}]") } else { "".to_string() }, (i + 1) * self.sram.wmask_granularity() - 1),
                                    TbSignals::Bl(i) => format!("N_X0/bl[{i}]_X0/Xcol_circuitry/Xcol_group_{}/Xprecharge_{}/Xbl_pull_up/M0_d", i / self.sram.mux_ratio(), i % self.sram.mux_ratio()),
                                    TbSignals::Br(i) => format!("N_X0/br[{i}]_X0/Xcol_circuitry/Xcol_group_{}/Xprecharge_{}/Xbr_pull_up/M0_d", i / self.sram.mux_ratio(), i % self.sram.mux_ratio()),
                                    TbSignals::BitcellQ(i,j) => format!("N_X0/Xbitcell_array/Xcell_{i}_{j}/X0/Q_X0/Xbitcell_array/Xcell_{i}_{j}/X0/X3/M0_s"),
                                    TbSignals::BitcellQB(i,j) => format!("N_X0/Xbitcell_array/Xcell_{i}_{j}/X0/QB_X0/Xbitcell_array/Xcell_{i}_{j}/X0/X4/M0_s"),
                                    TbSignals::WlCtlQ => "N_X0/Xcontrol_logic/Xwl_ctl/q0_X0/Xcontrol_logic/Xwl_ctl/Xnand_set/X0/X1/M0_d".to_string(),
                                    TbSignals::WlCtlQB => "N_X0/Xcontrol_logic/Xwl_ctl/q0b_X0/Xcontrol_logic/Xwl_ctl/Xnand_set/X0/X1/M0_g".to_string(),
                                    TbSignals::SaenCtlQ => "N_X0/Xcontrol_logic/Xsaen_ctl/q0_X0/Xcontrol_logic/Xsaen_ctl/Xnand_set/X0/X1/M0_d".to_string(),
                                    TbSignals::SaenCtlQB => "N_X0/Xcontrol_logic/Xsaen_ctl/q0b_X0/Xcontrol_logic/Xsaen_ctl/Xnand_set/X0/X1/M0_g".to_string(),
                                    TbSignals::PcCtlQ => "N_X0/Xcontrol_logic/Xpc_ctl/q0_X0/Xcontrol_logic/Xpc_ctl/Xnand_set/X0/X1/M0_d".to_string(),
                                    TbSignals::PcCtlQB => "N_X0/Xcontrol_logic/Xpc_ctl/q0b_X0/Xcontrol_logic/Xpc_ctl/Xnand_set/X0/X1/M0_g".to_string(),
                                    TbSignals::WrdrvenCtlQ => "N_X0/Xcontrol_logic/Xwrdrven_ctl/q0_X0/Xcontrol_logic/Xwrdrven_ctl/Xnand_set/X0/X1/M0_d".to_string(),
                                    TbSignals::WrdrvenCtlQB => "N_X0/Xcontrol_logic/Xwrdrven_ctl/q0b_X0/Xcontrol_logic/Xwrdrven_ctl/Xnand_set/X0/X1/M0_g".to_string(),
                                    TbSignals::DffsQ1(i) => format!("N_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_331_392#_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/X36/M0_s"),
                                    TbSignals::DffsQ1B(i) => format!("N_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_298_294#_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/X8/M0_s"),
                                    TbSignals::DffsQ2(i) => format!("N_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_1586_149#_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/X4/M0_s"),
                                    TbSignals::DffsQ2B(i) => format!("N_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_1800_291#_X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/X28/M0_s"),
                                    TbSignals::Wlen0 => "N_X0/wl_en0_X0/Xcontrol_logic/Xand_wlen/X0/X9/M0_s".to_string(),
                                    TbSignals::PcB0 => "N_X0/pc_b0_X0/Xcontrol_logic/Xpc_ctl/Xqb_inv/X0/X3/M0_s".to_string(),
                                    TbSignals::SenseEn0 => "N_X0/sense_en0_X0/Xcontrol_logic/Xsaen_ctl/Xq_inv/X0/X3/M0_s".to_string(),
                                    TbSignals::WriteDriverEn0 => "N_X0/write_driver_en0_X0/Xcontrol_logic/Xwrdrven_ctl/Xq_inv/X0/X3/M0_s".to_string(),
                                    TbSignals::WlBs => "N_X0/wl_b*_X0/Xdecoder/X0_2/Xgate_*_*_0/XMN0/M0_*".to_string(),
                                    TbSignals::BlOut(i) => {
                                        format!( "N_X0/Xcol_circuitry/Xcol_group_{i}/bl_out_X0/Xcol_circuitry/Xcol_group_{i}/Xsense_amp/X0/XINP/M0_g")
                                    }
                                    TbSignals::BrOut(i) => {
                                        format!( "N_X0/Xcol_circuitry/Xcol_group_{i}/br_out_X0/Xcol_circuitry/Xcol_group_{i}/Xsense_amp/X0/XINN/M0_g")
                                    }
                                    TbSignals::AddrGated(i) => format!( "N_X0/addr_gated[{i}]_X0/Xaddr_gate/Xgate_0_{i}_0/X0_1/XMP1/M0_d"),
                                    TbSignals::AddrBGated(i) => format!( "N_X0/addr_b_gated[{i}]_X0/Xaddr_gate/Xgate_0_{}_0/X0_1/XMP1/M0_d", i + self.sram.row_bits()),
                                    TbSignals::ColSelStart(i) => format!( "N_X0/col_sel[{i}]_X0/Xcol_circuitry/Xcol_group_0/Xmux_{i}/XMNBR/M0_g"),
                                    TbSignals::ColSelEnd(i) => format!( "N_X0/col_sel[{i}]_X0/Xcol_circuitry/Xcol_group_{}/Xmux_{i}/XMNBR/M0_g", self.sram.data_width() - 1),
                                    TbSignals::ColSelBStart(i) => format!( "N_X0/col_sel_b[{i}]_X0/Xcol_circuitry/Xcol_group_0/Xmux_{i}/XMPBR/M0_g"),
                                    TbSignals::ColSelBEnd(i) => format!( "N_X0/col_sel_b[{i}]_X0/Xcol_circuitry/Xcol_group_{}/Xmux_{i}/XMPBR/M0_g", self.sram.data_width() - 1),
                                    TbSignals::ChildConnsStart => "N_X0/Xdecoder/*child_conn_*_X0/Xdecoder/*/Xgate_0_*_0/X0_1/XMN1/M0_d".to_string(),
                                    TbSignals::ChildConnsEnd => "N_X0/Xdecoder/*child_conn_*_X0/Xdecoder/*/Xgate_0_*/M0_g".to_string(),
                                    TbSignals::LastStageDecoderXs => "N_X0/*/x_*X0/Xdecoder/X0_2/Xgate_*_0/XMN1/*".to_string(),
                                    TbSignals::LastStageDecoderVdds => "N_vdd_X0/Xdecoder/X0_2/Xgate_*_0/XMP0/M0_s".to_string(),
                                    TbSignals::LastStageDecoderVsss => "N_vss_X0/Xdecoder/X0_2/Xgate_*_0/XMN0/M0_s".to_string(),
                                    TbSignals::ColumnVdds => "N_vdd_X0/Xcol_circuitry/Xcol_group_*/*".to_string(),
                                    TbSignals::ColumnVsss => "N_vss_X0/Xcol_circuitry/Xcol_group_*/*".to_string(),
                                    TbSignals::WriteDriverQ(i) => {
                                        format!("N_X0/Xcol_circuitry/Xcol_group_{i}/q_X0/Xcol_circuitry/Xcol_group_{i}/Xwrite_driver/Xbrdriver/Xmn_pd/M0_g")
                                    }
                                    TbSignals::WriteDriverQB(i) => {
                                        format!("N_X0/Xcol_circuitry/Xcol_group_{i}/q_b_X0/Xcol_circuitry/Xcol_group_{i}/Xwrite_driver/Xbldriver/Xmn_pd/M0_g")
                                    }
                                    TbSignals::WmaskQ(i) => {
                                        format!("N_X0/Xcol_circuitry/wmask_in[{i}]_X0/Xcol_circuitry/Xwmask_dffs/Xdff_{i}/X0/X32/M0_d")
                                    }
                                }
                            }
                            PexLevel::C => {
                                match signal {
                                    TbSignals::Clk
                                    | TbSignals::We
                                    | TbSignals::Ce
                                    | TbSignals::RstB
                                    | TbSignals::Addr(_)
                                    | TbSignals::Wmask(_)
                                    | TbSignals::Din(_)
                                    | TbSignals::Dout(_) => unreachable!(),
                                    TbSignals::Wlen => "X0/wl_en".to_string(),
                                    TbSignals::Decrepstart => {
                                        "X0/Xcontrol_logic/decrepstart".to_string()
                                    }
                                    TbSignals::Decrepend => {
                                        "X0/Xcontrol_logic/decrepend".to_string()
                                    }
                                    TbSignals::PcBStart | TbSignals::PcBEnd => {
                                        "X0/pc_b".to_string()
                                    }
                                    TbSignals::SenseEnStart | TbSignals::SenseEnEnd => {
                                        "X0/sense_en".to_string()
                                    }
                                    TbSignals::Rwl => "X0/rwl".to_string(),
                                    TbSignals::Rbl => "X0/rbl".to_string(),
                                    TbSignals::WriteDriverEnStart | TbSignals::WriteDriverEnEnd => {
                                        "X0/write_driver_en".to_string()
                                    }
                                    TbSignals::WlStart(i) | TbSignals::WlEnd(i) => {
                                        format!("X0/wl[{i}]")
                                    }
                                    TbSignals::WeI(i) => format!(
                                        "X0/Xcol_circuitry/we_i{}",
                                        if self.sram.wmask_width() > 1 {
                                            format!("[{i}]")
                                        } else {
                                            "".to_string()
                                        }
                                    ),
                                    TbSignals::WeIb(i) => format!(
                                        "X0/Xcol_circuitry/we_ib{}",
                                        if self.sram.wmask_width() > 1 {
                                            format!("[{i}]")
                                        } else {
                                            "".to_string()
                                        }
                                    ),
                                    TbSignals::Bl(i) => format!("X0/bl[{i}]"),
                                    TbSignals::Br(i) => format!("X0/br[{i}]"),
                                    TbSignals::BitcellQ(i, j) => {
                                        format!("X0/Xbitcell_array/Xcell_{i}_{j}/X0/Q")
                                    }
                                    TbSignals::BitcellQB(i, j) => {
                                        format!("X0/Xbitcell_array/Xcell_{i}_{j}/X0/QB")
                                    }
                                    TbSignals::WlCtlQ => "X0/Xcontrol_logic/Xwl_ctl/q0".to_string(),
                                    TbSignals::WlCtlQB => {
                                        "X0/Xcontrol_logic/Xwl_ctl/q0b".to_string()
                                    }
                                    TbSignals::SaenCtlQ => {
                                        "X0/Xcontrol_logic/Xsaen_ctl/q0".to_string()
                                    }
                                    TbSignals::SaenCtlQB => {
                                        "X0/Xcontrol_logic/Xsaen_ctl/q0b".to_string()
                                    }
                                    TbSignals::PcCtlQ => "X0/Xcontrol_logic/Xpc_ctl/q0".to_string(),
                                    TbSignals::PcCtlQB => {
                                        "X0/Xcontrol_logic/Xpc_ctl/q0b".to_string()
                                    }
                                    TbSignals::WrdrvenCtlQ => {
                                        "X0/Xcontrol_logic/Xwrdrven_ctl/q0".to_string()
                                    }
                                    TbSignals::WrdrvenCtlQB => {
                                        "X0/Xcontrol_logic/Xwrdrven_ctl/q0b".to_string()
                                    }
                                    TbSignals::DffsQ1(i) => {
                                        format!("X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_331_392#")
                                    }
                                    TbSignals::DffsQ1B(i) => {
                                        format!("X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_298_294#")
                                    }
                                    TbSignals::DffsQ2(i) => {
                                        format!("X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_1586_149#")
                                    }
                                    TbSignals::DffsQ2B(i) => {
                                        format!("X0/Xaddr_we_ce_dffs/Xdff_{i}/X0/a_1800_291#")
                                    }
                                    TbSignals::Wlen0 => "X0/wl_en0".to_string(),
                                    TbSignals::PcB0 => "X0/pc_b0".to_string(),
                                    TbSignals::SenseEn0 => "X0/sense_en0".to_string(),
                                    TbSignals::WriteDriverEn0 => "X0/write_driver_en0".to_string(),
                                    TbSignals::WlBs => "X0/wl_b[*]".to_string(),
                                    TbSignals::BlOut(i) => {
                                        format!("X0/Xcol_circuitry/Xcol_group_{i}/bl_out")
                                    }
                                    TbSignals::BrOut(i) => {
                                        format!("X0/Xcol_circuitry/Xcol_group_{i}/br_out")
                                    }
                                    TbSignals::AddrGated(i) => format!("X0/addr_gated[{i}]"),
                                    TbSignals::AddrBGated(i) => format!("X0/addr_b_gated[{i}]"),
                                    TbSignals::ColSelStart(i) | TbSignals::ColSelEnd(i) => {
                                        format!("X0/col_sel[{i}]")
                                    }
                                    TbSignals::ColSelBStart(i) | TbSignals::ColSelBEnd(i) => {
                                        format!("X0/col_sel_b[{i}]")
                                    }
                                    TbSignals::ChildConnsStart | TbSignals::ChildConnsEnd => {
                                        "X0/Xdecoder/*child_conn_*".to_string()
                                    }
                                    TbSignals::LastStageDecoderXs => "X0/*/x_*".to_string(),
                                    TbSignals::LastStageDecoderVdds => "vdd".to_string(),
                                    TbSignals::LastStageDecoderVsss => "vdd".to_string(), // Hack, nothing
                                    // to save for vss
                                    TbSignals::ColumnVdds => "vdd".to_string(),
                                    TbSignals::ColumnVsss => "vdd".to_string(), // Hack, nothing to
                                    // save for vss
                                    TbSignals::WriteDriverQ(i) => {
                                        format!("X0/Xcol_circuitry/Xcol_group_{i}/q")
                                    }
                                    TbSignals::WriteDriverQB(i) => {
                                        format!("X0/Xcol_circuitry/Xcol_group_{i}/q_b")
                                    }
                                    TbSignals::WmaskQ(i) => {
                                        format!("X0/Xcol_circuitry/wmask_in[{i}]")
                                    }
                                }
                            }
                            _ => unimplemented!(),
                        }
                    )
                } else {
                    format!(
                        "Xdut.X0.{}",
                        match signal {
                            TbSignals::Clk
                            | TbSignals::We
                            | TbSignals::Ce
                            | TbSignals::RstB
                            | TbSignals::Addr(_)
                            | TbSignals::Wmask(_)
                            | TbSignals::Din(_)
                            | TbSignals::Dout(_) => unreachable!(),
                            TbSignals::Wlen => "wl_en".to_string(),
                            TbSignals::Decrepstart => "Xcontrol_logic.decrepstart".to_string(),
                            TbSignals::Decrepend => "Xcontrol_logic.decrepend".to_string(),
                            TbSignals::PcBStart | TbSignals::PcBEnd => "pc_b".to_string(),
                            TbSignals::SenseEnStart | TbSignals::SenseEnEnd =>
                                "sense_en".to_string(),
                            TbSignals::Rwl => "rwl".to_string(),
                            TbSignals::Rbl => "rbl".to_string(),
                            TbSignals::WriteDriverEnStart | TbSignals::WriteDriverEnEnd =>
                                "write_driver_en".to_string(),
                            TbSignals::WlStart(i) | TbSignals::WlEnd(i) => format!("wl[{i}]"),
                            TbSignals::WeI(i) => format!(
                                "Xcol_circuitry.we_i{}",
                                if self.sram.wmask_width() > 1 {
                                    format!("[{i}]")
                                } else {
                                    "".to_string()
                                }
                            ),
                            TbSignals::WeIb(i) => format!(
                                "Xcol_circuitry.we_ib{}",
                                if self.sram.wmask_width() > 1 {
                                    format!("[{i}]")
                                } else {
                                    "".to_string()
                                }
                            ),
                            TbSignals::Bl(i) => format!("bl[{i}]"),
                            TbSignals::Br(i) => format!("br[{i}]"),
                            TbSignals::BitcellQ(i, j) =>
                                format!("Xbitcell_array.Xcell_{i}_{j}.X0.Q"),
                            TbSignals::BitcellQB(i, j) =>
                                format!("Xbitcell_array.Xcell_{i}_{j}.X0.QB"),
                            TbSignals::WlCtlQ => "Xcontrol_logic.Xwl_ctl.q0".to_string(),
                            TbSignals::WlCtlQB => "Xcontrol_logic.Xwl_ctl.q0b".to_string(),
                            TbSignals::SaenCtlQ => "Xcontrol_logic.Xsaen_ctl.q0".to_string(),
                            TbSignals::SaenCtlQB => "Xcontrol_logic.Xsaen_ctl.q0b".to_string(),
                            TbSignals::PcCtlQ => "Xcontrol_logic.Xpc_ctl.q0".to_string(),
                            TbSignals::PcCtlQB => "Xcontrol_logic.Xpc_ctl.q0b".to_string(),
                            TbSignals::WrdrvenCtlQ => "Xcontrol_logic.Xwrdrven_ctl.q0".to_string(),
                            TbSignals::WrdrvenCtlQB =>
                                "Xcontrol_logic.Xwrdrven_ctl.q0b".to_string(),
                            TbSignals::DffsQ1(i) =>
                                format!("Xaddr_we_ce_dffs.Xdff_{i}.X0.a_331_392#"),
                            TbSignals::DffsQ1B(i) =>
                                format!("Xaddr_we_ce_dffs.Xdff_{i}.X0.a_298_294#"),
                            TbSignals::DffsQ2(i) =>
                                format!("Xaddr_we_ce_dffs.Xdff_{i}.X0.a_1586_149#"),
                            TbSignals::DffsQ2B(i) =>
                                format!("Xaddr_we_ce_dffs.Xdff_{i}.X0.a_1800_291#"),
                            TbSignals::Wlen0 => "wl_en0".to_string(),
                            TbSignals::PcB0 => "pc_b0".to_string(),
                            TbSignals::SenseEn0 => "sense_en0".to_string(),
                            TbSignals::WriteDriverEn0 => "write_driver_en0".to_string(),
                            TbSignals::WlBs => "wl_b[*]".to_string(),
                            TbSignals::BlOut(i) => format!("Xcol_circuitry.Xcol_group_{i}.bl_out"),
                            TbSignals::BrOut(i) => format!("Xcol_circuitry.Xcol_group_{i}.br_out"),
                            TbSignals::AddrGated(i) => format!("addr_gated[{i}]"),
                            TbSignals::AddrBGated(i) => format!("addr_b_gated[{i}]"),
                            TbSignals::ColSelStart(i) | TbSignals::ColSelEnd(i) =>
                                format!("col_sel[{i}]"),
                            TbSignals::ColSelBStart(i) | TbSignals::ColSelBEnd(i) =>
                                format!("col_sel_b[{i}]"),
                            TbSignals::ChildConnsStart | TbSignals::ChildConnsEnd =>
                                "Xdecoder.*child_conn_*".to_string(),
                            TbSignals::LastStageDecoderXs => "*.x_*".to_string(),
                            TbSignals::LastStageDecoderVdds => "vdd".to_string(),
                            TbSignals::LastStageDecoderVsss => "vdd".to_string(), // Hack, nothing
                            // to save for vss
                            TbSignals::ColumnVdds => "vdd".to_string(),
                            TbSignals::ColumnVsss => "vdd".to_string(), // Hack, nothing to
                            // save for vss
                            TbSignals::WriteDriverQ(i) =>
                                format!("Xcol_circuitry.Xcol_group_{i}.q"),
                            TbSignals::WriteDriverQB(i) =>
                                format!("Xcol_circuitry.Xcol_group_{i}.q_b"),
                            TbSignals::WmaskQ(i) => {
                                format!("Xcol_circuitry.wmask_in[{i}]")
                            }
                        }
                    )
                }
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Reset,
    None,
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

    /// Chip enable.
    ce: Waveform,

    /// Write enable.
    we: Waveform,

    /// Reset.
    rstb: Waveform,

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
    let mut addr = vec![Waveform::with_initial_value(0f64); params.sram.addr_width()];
    let mut din = vec![Waveform::with_initial_value(0f64); params.sram.data_width()];
    let wmask_bits = params.sram.wmask_width();
    let mut wmask = vec![Waveform::with_initial_value(0f64); wmask_bits];
    let mut clk = Waveform::with_initial_value(0f64);
    let mut ce = Waveform::with_initial_value(0f64);
    let mut we = Waveform::with_initial_value(0f64);
    let mut rstb = Waveform::with_initial_value(0f64);

    let period = params.clk_period;
    let vdd = params.vdd;
    let tr = params.tr;
    let tf = params.tf;

    let mut t = 0f64;
    let mut t_end;

    let wmask_all = BitSignal::ones(params.sram.wmask_width());

    for op in params.ops.iter() {
        t_end = t + period;
        let t_data = t_end + params.t_hold;
        // Toggle the clock
        clk.push_high(t + (period / 2.0), vdd, tr);
        clk.push_low(t + period, vdd, tf);

        match op {
            Op::Reset => {
                // Set write enable low
                we.push_low(t_data, vdd, tf);
                // Set chip enable low
                ce.push_low(t_data, vdd, tf);
                // Set reset high
                rstb.push_low(t_data + period / 2., vdd, tf);
            }
            Op::None => {
                // Set write enable low
                we.push_low(t_data, vdd, tf);
                // Set chip enable low
                ce.push_low(t_data, vdd, tf);
                // Set reset low
                rstb.push_high(t_data, vdd, tr);
            }
            Op::Read { addr: addrv } => {
                // Set write enable low
                we.push_low(t_data, vdd, tf);
                // Set chip enable high
                ce.push_high(t_data, vdd, tr);
                // Set reset low
                rstb.push_high(t_data, vdd, tr);

                assert_eq!(addrv.width(), params.sram.addr_width());
                push_bus(&mut addr, addrv, t_data, vdd, tr, tf);
            }
            Op::Write { addr: addrv, data } => {
                // Set write enable high
                we.push_high(t_data, vdd, tr);
                // Set chip enable high
                ce.push_high(t_data, vdd, tr);
                // Set reset low
                rstb.push_high(t_data, vdd, tr);

                assert_eq!(addrv.width(), params.sram.addr_width());
                push_bus(&mut addr, addrv, t_data, vdd, tr, tf);

                assert_eq!(data.width(), params.sram.data_width);
                push_bus(&mut din, data, t_data, vdd, tr, tf);

                push_bus(&mut wmask, &wmask_all, t_data, vdd, tr, tf);
            }

            Op::WriteMasked {
                addr: addrv,
                data,
                mask,
            } => {
                // Set write enable high
                we.push_high(t_data, vdd, tr);
                // Set chip enable high
                ce.push_high(t_data, vdd, tr);
                // Set reset low
                rstb.push_high(t_data, vdd, tr);

                assert_eq!(addrv.width(), params.sram.addr_width());
                push_bus(&mut addr, addrv, t_data, vdd, tr, tf);

                assert_eq!(data.width(), params.sram.data_width);
                push_bus(&mut din, data, t_data, vdd, tr, tf);

                assert!(params.sram.wmask_width() > 1);
                assert_eq!(mask.width(), params.sram.wmask_width());
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
        ce,
        we,
        rstb,
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
        let [vdd, clk, ce, we, rstb] = ctx.signals(["vdd", "clk", "ce", "we", "rstb"]);

        let addr = ctx.bus("addr", self.params.sram.addr_width());
        let din = ctx.bus("din", self.params.sram.data_width());
        let dout = ctx.bus("dout", self.params.sram.data_width());
        let wmask = ctx.bus("wmask", self.params.sram.wmask_width());

        let waveforms = generate_waveforms(&self.params);
        let output_cap = SiValue::with_precision(self.params.c_load, SiPrefix::Femto);

        if let Some((ref pex_netlist, _)) = self.params.pex_netlist {
            ctx.instantiate::<SramPex>(&SramPexParams {
                params: self.params.sram.clone(),
                pex_netlist: pex_netlist.clone(),
            })?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("clk", clk),
                ("ce", ce),
                ("we", we),
                ("rstb", rstb),
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
                    ("ce", ce),
                    ("we", we),
                    ("rstb", rstb),
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
        ctx.instantiate::<Vpwl>(&Arc::new(waveforms.ce))?
            .with_connections([("p", ce), ("n", vss)])
            .named("Vce")
            .add_to(ctx);
        ctx.instantiate::<Vpwl>(&Arc::new(waveforms.we))?
            .with_connections([("p", we), ("n", vss)])
            .named("Vwe")
            .add_to(ctx);
        ctx.instantiate::<Vpwl>(&Arc::new(waveforms.rstb))?
            .with_connections([("p", rstb), ("n", vss)])
            .named("Vrstb")
            .add_to(ctx);
        for i in 0..self.params.sram.addr_width() {
            ctx.instantiate::<Vpwl>(&Arc::new(waveforms.addr[i].clone()))?
                .with_connections([("p", addr.index(i)), ("n", vss)])
                .named(format!("Vaddr_{i}"))
                .add_to(ctx);
        }
        for i in 0..self.params.sram.wmask_width() {
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

fn bits0101(width: usize) -> Vec<bool> {
    alternating_bits(width, true)
}

fn bits1010(width: usize) -> Vec<bool> {
    alternating_bits(width, false)
}

fn alternating_bits(width: usize, start: bool) -> Vec<bool> {
    let mut bit = start;
    let mut bits = Vec::with_capacity(width);
    for _ in 0..width {
        bits.push(bit);
        bit = !bit;
    }
    bits
}

pub fn march_cm_test(params: SramParams) -> Vec<Op> {
    let n = params.num_words() as u64;
    let aw = params.addr_width();
    let dw = params.data_width();
    (0..n)
        .map(|i| Op::Write {
            addr: BitSignal::from_u64(i, aw),
            data: BitSignal::zeros(dw),
        })
        .chain((0..n).flat_map(|i| {
            [
                Op::Read {
                    addr: BitSignal::from_u64(i, aw),
                },
                Op::Write {
                    addr: BitSignal::from_u64(i, aw),
                    data: BitSignal::ones(dw),
                },
            ]
        }))
        .chain((0..n).flat_map(|i| {
            [
                Op::Read {
                    addr: BitSignal::from_u64(i, aw),
                },
                Op::Write {
                    addr: BitSignal::from_u64(i, aw),
                    data: BitSignal::zeros(dw),
                },
            ]
        }))
        .chain((0..n).rev().flat_map(|i| {
            [
                Op::Read {
                    addr: BitSignal::from_u64(i, aw),
                },
                Op::Write {
                    addr: BitSignal::from_u64(i, aw),
                    data: BitSignal::ones(dw),
                },
            ]
        }))
        .chain((0..n).rev().flat_map(|i| {
            [
                Op::Read {
                    addr: BitSignal::from_u64(i, aw),
                },
                Op::Write {
                    addr: BitSignal::from_u64(i, aw),
                    data: BitSignal::zeros(dw),
                },
            ]
        }))
        .chain((0..n).rev().map(|i| Op::Read {
            addr: BitSignal::from_u64(i, aw),
        }))
        .collect::<Vec<_>>()
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TestSequence {
    Short,
    Medium,
    MarchCm,
}

impl TestSequence {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestSequence::Short => "short",
            TestSequence::Medium => "medium",
            TestSequence::MarchCm => "marchcm",
        }
    }
}

impl Display for TestSequence {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn tb_params(
    params: SramParams,
    dsn: Arc<SramPhysicalDesign>,
    vdd: f64,
    sequence: TestSequence,
    pex_netlist: Option<(PathBuf, PexLevel)>,
) -> TbParams {
    let wmask_width = params.wmask_width();
    let data_width = params.data_width();
    let addr_width = params.addr_width();

    // An alternating 64-bit sequence 0b010101...01
    let bit_pattern1 = 0x5555555555555555u128;

    // An alternating 64-bit sequence 0b101010...10
    let bit_pattern2 = 0xAAAAAAAAAAAAAAAAu128;

    let addr1 = BitSignal::zeros(addr_width);
    let addr2 = BitSignal::ones(addr_width);

    let mut short_ops = vec![
        Op::Reset,
        Op::Write {
            addr: addr1.clone(),
            data: BitSignal::from_vec(bits0101(data_width)),
        },
        Op::None,
        Op::Write {
            addr: addr2.clone(),
            data: BitSignal::from_vec(bits1010(data_width)),
        },
        Op::None,
        Op::None,
        Op::Read {
            addr: addr1.clone(),
        },
        Op::Read { addr: addr2 },
        Op::Read { addr: addr1 },
    ];

    let ops = match sequence {
        TestSequence::Short => short_ops,
        TestSequence::Medium => {
            for i in 0..16 {
                let bits = (i % 2) * bit_pattern2 + (1 - (i % 2)) * bit_pattern1 + i + 1;
                short_ops.push(Op::Write {
                    addr: BitSignal::from_u128(i, addr_width),
                    data: BitSignal::from_u128_padded(bits, data_width),
                });
            }
            for i in 0..16 {
                short_ops.push(Op::Read {
                    addr: BitSignal::from_u128(i, addr_width),
                });
            }

            if wmask_width > 1 {
                for i in 0..16 {
                    let bits = (1 - (i % 2)) * bit_pattern2 + (i % 2) * bit_pattern1 + i + 1;
                    short_ops.push(Op::WriteMasked {
                        addr: BitSignal::from_u128(i, addr_width),
                        data: BitSignal::from_u128_padded(bits, data_width),
                        mask: BitSignal::from_u128_padded(
                            bit_pattern1 + i * 0b10110010111,
                            wmask_width,
                        ),
                    });
                }
                for i in 0..16 {
                    short_ops.push(Op::Read {
                        addr: BitSignal::from_u128(i, addr_width),
                    });
                }
            }

            short_ops
        }
        TestSequence::MarchCm => march_cm_test(params),
    };

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
        .dsn(dsn)
        .pex_netlist(pex_netlist)
        .build()
        .unwrap();

    tb
}

impl Testbench for SramTestbench {
    type Output = TranData;
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
        if let Some((ref netlist, _)) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        ctx.add_analysis(
            TranAnalysis::builder()
                .stop(wav.clk.last_t().unwrap() + 2.0 * step)
                // .stop(80e-9)
                .step(step)
                // .strobe_period(step)
                .opts(opts)
                .build()
                .unwrap(),
        );

        let signals = (0..self.params.sram.data_width)
            .flat_map(|i| {
                [
                    TbSignals::Din(i),
                    TbSignals::Dout(i),
                    TbSignals::WriteDriverQ(i),
                    TbSignals::WriteDriverQB(i),
                ]
            })
            .chain([
                TbSignals::Clk,
                TbSignals::We,
                TbSignals::Ce,
                TbSignals::RstB,
                TbSignals::Wlen,
                TbSignals::Wlen0,
                TbSignals::Decrepstart,
                TbSignals::Decrepend,
                TbSignals::PcBStart,
                TbSignals::PcBEnd,
                TbSignals::PcB0,
                TbSignals::SenseEnStart,
                TbSignals::SenseEnEnd,
                TbSignals::SenseEn0,
                TbSignals::Rwl,
                TbSignals::Rbl,
                TbSignals::WriteDriverEnStart,
                TbSignals::WriteDriverEnEnd,
                TbSignals::WriteDriverEn0,
                TbSignals::WlBs,
                TbSignals::ChildConnsStart,
                TbSignals::ChildConnsEnd,
                TbSignals::LastStageDecoderXs,
                TbSignals::LastStageDecoderVdds,
                TbSignals::LastStageDecoderVsss,
                TbSignals::ColumnVdds,
                TbSignals::ColumnVsss,
            ])
            .chain(
                (0..self.params.sram.rows())
                    .flat_map(|i| [TbSignals::WlStart(i), TbSignals::WlEnd(i)]),
            )
            .chain((0..self.params.sram.addr_width()).map(|i| TbSignals::Addr(i)))
            .chain((0..self.params.sram.wmask_width()).flat_map(|i| {
                [
                    TbSignals::WeI(i),
                    TbSignals::WeIb(i),
                    TbSignals::Wmask(i),
                    TbSignals::WmaskQ(i),
                ]
            }))
            .chain((0..self.params.sram.cols()).flat_map(|i| {
                [
                    TbSignals::Bl(i),
                    TbSignals::Br(i),
                    TbSignals::BlOut(i),
                    TbSignals::BrOut(i),
                ]
            }))
            .chain(
                (0..self.params.sram.row_bits())
                    .flat_map(|i| [TbSignals::AddrGated(i), TbSignals::AddrBGated(i)]),
            )
            .chain((0..self.params.sram.mux_ratio()).flat_map(|i| {
                [
                    TbSignals::ColSelStart(i),
                    TbSignals::ColSelEnd(i),
                    TbSignals::ColSelBStart(i),
                    TbSignals::ColSelBEnd(i),
                ]
            }))
            .map(|signal| self.params.sram_signal_path(signal))
            .collect::<HashSet<_>>();

        ctx.save(Save::Signals(signals));
        // ctx.save(Save::All);

        let vdd = SiValue::with_precision(self.params.vdd, SiPrefix::Nano);

        for i in 0..self.params.sram.rows() {
            ctx.set_ic(
                self.params.sram_signal_path(TbSignals::WlStart(i)),
                SiValue::zero(),
            );
            for j in 0..self.params.sram.cols() {
                ctx.set_ic(
                    self.params.sram_signal_path(TbSignals::BitcellQ(i, j)),
                    SiValue::zero(),
                );
                ctx.set_ic(
                    self.params.sram_signal_path(TbSignals::BitcellQB(i, j)),
                    vdd,
                );
            }
        }
        for signal in [
            TbSignals::WlCtlQ,
            TbSignals::PcCtlQB,
            TbSignals::SaenCtlQ,
            TbSignals::WrdrvenCtlQ,
        ] {
            ctx.set_ic(self.params.sram_signal_path(signal), SiValue::zero());
        }
        for signal in [
            TbSignals::WlCtlQB,
            TbSignals::PcCtlQ,
            TbSignals::SaenCtlQB,
            TbSignals::WrdrvenCtlQB,
        ] {
            ctx.set_ic(self.params.sram_signal_path(signal), vdd);
        }
        for i in 0..self.params.sram.addr_width() + 2 {
            ctx.set_ic(
                self.params.sram_signal_path(TbSignals::DffsQ1(i)),
                SiValue::zero(),
            );
            ctx.set_ic(
                self.params.sram_signal_path(TbSignals::DffsQ2(i)),
                SiValue::zero(),
            );
            ctx.set_ic(self.params.sram_signal_path(TbSignals::DffsQ1B(i)), vdd);
            ctx.set_ic(self.params.sram_signal_path(TbSignals::DffsQ2B(i)), vdd);
        }
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        let data = ctx.output().data[0].tran();
        Ok(data.clone())
    }
}
