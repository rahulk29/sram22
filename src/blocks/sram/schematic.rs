use substrate::component::NoParams;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::signal::Signal;

use crate::blocks::bitcell_array::replica::{ReplicaCellArray, ReplicaCellArrayParams};
use crate::blocks::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::blocks::columns::{ColParams, ColPeripherals};
use crate::blocks::control::{ControlLogicReplicaV2, DffArray, InvChain};
use crate::blocks::decoder::layout::LastBitDecoderStage;
use crate::blocks::decoder::{
    AddrGate, AddrGateParams, Decoder, DecoderParams, DecoderStageParams, DecoderTree, INV_MODEL,
    INV_PARAMS, NAND2_PARAMS,
};
use crate::blocks::gate::sizing::InverterGateTreeNode;
use crate::blocks::gate::{AndParams, GateParams, PrimitiveGateParams};
use crate::blocks::precharge::{Precharge, PrechargeParams};
use crate::blocks::tgatemux::TGateMuxParams;
use crate::blocks::wrdriver::WriteDriverParams;

use super::{SramInner, SramPhysicalDesignScript, READ_MUX_INPUT_CAP, WORDLINE_CAP_PER_CELL};

impl SramInner {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<SramPhysicalDesignScript>(&self.params)?;
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [clk, we, ce, reset_b] = ctx.ports(["clk", "we", "ce", "reset_b"], Direction::Input);

        let addr = ctx.bus_port("addr", self.params.addr_width(), Direction::Input);
        let wmask = ctx.bus_port("wmask", self.params.wmask_width(), Direction::Input);
        let din = ctx.bus_port("din", self.params.data_width(), Direction::Input);
        let dout = ctx.bus_port("dout", self.params.data_width(), Direction::Output);

        let [addr_in0, addr_in, addr_in0_b, addr_in_b] = ctx.buses(
            ["addr_in0", "addr_in", "addr_in0_b", "addr_in_b"],
            self.params.addr_width(),
        );

        let [addr_gated, addr_b_gated] =
            ctx.buses(["addr_gated", "addr_b_gated"], self.params.row_bits());

        let bl = ctx.bus("bl", self.params.cols());
        let br = ctx.bus("br", self.params.cols());
        let wl = ctx.bus("wl", self.params.rows());
        let wl_b = ctx.bus("wl_b", self.params.rows());

        let col_sel0 = ctx.bus("col_sel0", self.params.mux_ratio());
        let col_sel = ctx.bus("col_sel", self.params.mux_ratio());
        let col_sel0_b = ctx.bus("col_sel0_b", self.params.mux_ratio());
        let col_sel_b = ctx.bus("col_sel_b", self.params.mux_ratio());

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;

        let diode = lib.try_cell_named("sky130_fd_sc_hs__diode_2")?;

        for (port, width) in [
            (dout, self.params.data_width()),
            (din, self.params.data_width()),
            (wmask, self.params.wmask_width()),
        ] {
            for i in 0..width {
                ctx.instantiate::<StdCell>(&diode.id())?
                    .with_connections([
                        ("DIODE", port.index(i)),
                        ("VPWR", vdd),
                        ("VPB", vdd),
                        ("VGND", vss),
                        ("VNB", vss),
                    ])
                    .add_to(ctx);
            }
        }

        let [we_in, we_in_b, ce_in, ce_in_b, dummy_bl, dummy_br, rwl, rbl, rbr, pc_b0, pc, pc_b, wl_en0, wl_en_b, wl_en, write_driver_en0, write_driver_en_b, write_driver_en, sense_en0, sense_en_b, sense_en] =
            ctx.signals([
                "we_in",
                "we_in_b",
                "ce_in",
                "ce_in_b",
                "dummy_bl",
                "dummy_br",
                "rwl",
                "rbl",
                "rbr",
                "pc_b0",
                "pc",
                "pc_b",
                "wl_en0",
                "wl_en_b",
                "wl_en",
                "write_driver_en0",
                "write_driver_en_b",
                "write_driver_en",
                "sense_en0",
                "sense_en_b",
                "sense_en",
            ]);

        let wl_cap = (self.params.cols() + 4) as f64 * WORDLINE_CAP_PER_CELL;

        ctx.instantiate::<AddrGate>(&dsn.addr_gate)?
            .with_connections([("vdd", vdd), ("vss", vss), ("wl_en", wl_en)])
            .with_connections([
                (
                    "in",
                    Signal::new(vec![
                        addr_in.index(self.params.col_select_bits()..),
                        addr_in_b.index(self.params.col_select_bits()..),
                    ]),
                ),
                ("y", Signal::new(vec![addr_gated, addr_b_gated])),
            ])
            .named("addr_gate")
            .add_to(ctx);

        ctx.instantiate::<Decoder>(&dsn.row_decoder)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("addr", addr_gated),
                ("addr_b", addr_b_gated),
                ("decode", wl),
                ("decode_b", wl_b),
            ])
            .named("decoder")
            .add_to(ctx);

        ctx.instantiate::<Decoder>(&dsn.col_decoder)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("addr", addr_in.index(0..self.params.col_select_bits())),
                ("addr_b", addr_in_b.index(0..self.params.col_select_bits())),
                ("decode", col_sel0),
                ("decode_b", col_sel0_b),
            ])
            .named("column_decoder")
            .add_to(ctx);

        let control_logic = ctx
            .instantiate::<ControlLogicReplicaV2>(&NoParams)?
            .with_connections([
                ("clk", clk),
                ("we", we_in),
                ("ce", ce_in),
                ("reset_b", reset_b),
                ("rbl", rbl),
                ("rwl", rwl),
                ("pc_b", pc_b0),
                ("wlen", wl_en0),
                ("wrdrven", write_driver_en0),
                ("saen", sense_en0),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("control_logic");
        control_logic.add_to(ctx);

        // TODO: estimate load capacitances
        ctx.instantiate::<LastBitDecoderStage>(&dsn.pc_b_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", pc_b),
                ("y_b", pc),
                ("predecode_0_0", pc_b0),
            ])
            .named("pc_b_buffer")
            .add_to(ctx);
        ctx.instantiate::<LastBitDecoderStage>(&dsn.wlen_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", wl_en),
                ("y_b", wl_en_b),
                ("predecode_0_0", wl_en0),
            ])
            .named("wlen_buffer")
            .add_to(ctx);
        ctx.instantiate::<LastBitDecoderStage>(&dsn.write_driver_en_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", write_driver_en),
                ("y_b", write_driver_en_b),
                ("predecode_0_0", write_driver_en0),
            ])
            .named("write_driver_en_buffer")
            .add_to(ctx);
        ctx.instantiate::<LastBitDecoderStage>(&dsn.sense_en_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", sense_en),
                ("y_b", sense_en_b),
                ("predecode_0_0", sense_en0),
            ])
            .named("sense_en_buffer")
            .add_to(ctx);

        ctx.instantiate::<DffArray>(&dsn.num_dffs)?
            .with_connections([("vdd", vdd), ("vss", vss), ("clk", clk), ("rb", reset_b)])
            .with_connection("d", Signal::new(vec![addr, we, ce]))
            .with_connection("q", Signal::new(vec![addr_in, we_in, ce_in]))
            .with_connection("qn", Signal::new(vec![addr_in_b, we_in_b, ce_in_b]))
            .named("addr_we_ce_dffs")
            .add_to(ctx);

        ctx.instantiate::<SpCellArray>(&dsn.bitcells)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("dummy_bl", dummy_bl),
                ("dummy_br", dummy_br),
                ("bl", bl),
                ("br", br),
                ("wl", wl),
            ])
            .named("bitcell_array")
            .add_to(ctx);

        ctx.instantiate::<ReplicaCellArray>(&dsn.rbl)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("rbl", rbl),
                ("rbr", rbr),
                ("rwl", rwl),
            ])
            .named("replica_bitcell_array")
            .add_to(ctx);

        ctx.instantiate::<ColPeripherals>(&dsn.col_params)?
            .with_connections([
                ("clk", clk),
                ("reset_b", reset_b),
                ("vdd", vdd),
                ("vss", vss),
                ("bl", bl),
                ("br", br),
                ("pc_b", pc_b),
                ("sel", col_sel),
                ("sel_b", col_sel_b),
                ("we", write_driver_en),
                ("wmask", wmask),
                ("din", din),
                ("dout", dout),
                ("sense_en", sense_en),
            ])
            .named("col_circuitry")
            .add_to(ctx);

        for i in 0..dsn.replica_pc.cols {
            ctx.instantiate::<Precharge>(&dsn.col_params.pc)?
                .with_connections([("vdd", vdd), ("bl", rbl), ("br", rbr), ("en_b", pc_b0)])
                .named(format!("replica_precharge_{i}"))
                .add_to(ctx);
        }

        Ok(())
    }
}

fn buffer_chain_num_stages(cl: f64) -> usize {
    let fo = cl / INV_MODEL.cin;
    if fo < 2.0 {
        return 2;
    }
    let stages = 2 * (fo.log(3.0) / 2.0).round() as usize;
    let stages = if stages == 0 { 2 } else { stages };

    assert_eq!(stages % 2, 0);
    stages
}

fn inverter_chain_num_stages(cl: f64) -> usize {
    let fo = cl / INV_MODEL.cin;
    if fo < 4.0 {
        return 1;
    }
    // round to odd
    let stages = 2 * ((fo.log(3.0) - 1.0) / 2.0).round() as usize + 1;
    let stages = if stages == 0 { 1 } else { stages };

    assert_eq!(stages % 2, 1);
    stages
}

pub fn fanout_buffer_stage(cl: f64) -> DecoderStageParams {
    let stages = buffer_chain_num_stages(cl);
    let invs = InverterGateTreeNode::buffer(stages)
        .elaborate()
        .size(cl)
        .as_inv_chain();
    DecoderStageParams {
        max_width: None,
        gate: GateParams::Inv(invs[0]),
        invs: invs.into_iter().skip(1).collect(),
        num: 1,
        child_sizes: vec![1],
    }
}

fn fanout_buffer_stage_with_inverted_output(cl: f64) -> DecoderStageParams {
    let stages = inverter_chain_num_stages(cl);
    let mut invs = InverterGateTreeNode::inverter(stages)
        .elaborate()
        .size(cl)
        .as_inv_chain();
    invs.push(invs.last().unwrap().clone());
    DecoderStageParams {
        max_width: None,
        gate: GateParams::Inv(invs[0]),
        invs: invs.into_iter().skip(1).collect(),
        num: 1,
        child_sizes: vec![1],
    }
}

#[cfg(test)]
mod tests {
    use crate::blocks::decoder::INV_MODEL;
    use crate::blocks::sram::schematic::buffer_chain_num_stages;

    #[test]
    fn test_num_stages() {
        assert_eq!(buffer_chain_num_stages(4. * INV_MODEL.cin), 2);
        assert_eq!(buffer_chain_num_stages(16. * INV_MODEL.cin), 2);
        assert_eq!(buffer_chain_num_stages(26. * INV_MODEL.cin), 2);
        assert_eq!(buffer_chain_num_stages(28. * INV_MODEL.cin), 4);
        assert_eq!(buffer_chain_num_stages(242. * INV_MODEL.cin), 4);
        assert_eq!(buffer_chain_num_stages(244. * INV_MODEL.cin), 6);
    }
}
