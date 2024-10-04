use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::signal::Signal;

use crate::blocks::bitcell_array::replica::{ReplicaCellArray, ReplicaCellArrayParams};
use crate::blocks::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::blocks::columns::{ColParams, ColPeripherals};
use crate::blocks::control::{ControlLogicParams, ControlLogicReplicaV2, DffArray, InvChain};
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

use super::{SramInner, READ_MUX_INPUT_CAP, WORDLINE_CAP_PER_CELL};

impl SramInner {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [clk, we, ce, reset_b] = ctx.ports(["clk", "we", "ce", "reset_b"], Direction::Input);

        let addr = ctx.bus_port("addr", self.params.addr_width(), Direction::Input);
        let wmask = ctx.bus_port("wmask", self.params.wmask_width(), Direction::Input);
        let din = ctx.bus_port("din", self.params.data_width(), Direction::Input);
        let dout = ctx.bus_port("dout", self.params.data_width(), Direction::Output);

        let [addr_in, addr_in_b] = ctx.buses(["addr_in", "addr_in_b"], self.params.addr_width());

        let [addr_gated0, addr_gated, addr_gated_b, addr_b_gated0, addr_b_gated, addr_b_gated_b] =
            ctx.buses(
                [
                    "addr_gated0",
                    "addr_gated",
                    "addr_gated_b",
                    "addr_b_gated0",
                    "addr_b_gated",
                    "addr_b_gated_b",
                ],
                self.params.row_bits(),
            );

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

        ctx.instantiate::<AddrGate>(&AddrGateParams {
            gate: GateParams::And2(AndParams {
                // TODO fix this
                nand: NAND2_PARAMS,
                inv: INV_PARAMS,
            }),
            num: self.params.row_bits(),
        })?
        .with_connections([
            ("vdd", vdd),
            ("vss", vss),
            ("addr", addr_in.index(self.params.col_select_bits()..)),
            ("addr_b", addr_in_b.index(self.params.col_select_bits()..)),
            ("addr_gated", addr_gated0),
            ("addr_b_gated", addr_b_gated0),
            ("en", wl_en),
        ])
        .named("addr_gate")
        .add_to(ctx);

        let wl_cap = (self.params.cols() + 4) as f64 * WORDLINE_CAP_PER_CELL;
        let tree = DecoderTree::new(self.params.row_bits(), 4. * wl_cap);
        let decoder_params = DecoderParams { tree };

        ctx.instantiate::<Decoder>(&decoder_params)?
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

        // TODO add wmux driver input capacitance
        let col_tree = DecoderTree::new(
            self.params.col_select_bits(),
            READ_MUX_INPUT_CAP * (self.params.cols() / self.params.mux_ratio()) as f64,
        );
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };

        ctx.instantiate::<Decoder>(&col_decoder_params)?
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

        let decoder_delay_invs = (f64::max(
            4.0,
            1.1 * decoder_params.tree.root.time_constant(wl_cap)
                / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)),
        ) / 2.0)
            .round() as usize
            * 2
            + 2;
        println!("decoder_delay_invs: {}", decoder_delay_invs);
        assert_eq!(decoder_delay_invs % 2, 0);
        let control_logic = ctx
            .instantiate::<ControlLogicReplicaV2>(&ControlLogicParams { decoder_delay_invs })?
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

        for i in 0..self.params.mux_ratio() {
            let buffer = fanout_buffer_stage_with_inverted_output(400e-15);
            ctx.instantiate::<LastBitDecoderStage>(&buffer)?
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("y", col_sel.index(i)),
                    ("y_b", col_sel_b.index(i)),
                    ("predecode_0_0", col_sel0.index(i)),
                ])
                .named(format!("col_sel_buf_{i}"))
                .add_to(ctx);
        }

        // TODO: estimate load capacitances
        let pc_b_buffer = fanout_buffer_stage(2000e-15);
        ctx.instantiate::<LastBitDecoderStage>(&pc_b_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", pc_b),
                ("y_b", pc),
                ("predecode_0_0", pc_b0),
            ])
            .named("pc_b_buffer")
            .add_to(ctx);
        let wlen_buffer = fanout_buffer_stage(400e-15);
        ctx.instantiate::<LastBitDecoderStage>(&wlen_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", wl_en),
                ("y_b", wl_en_b),
                ("predecode_0_0", wl_en0),
            ])
            .named("wlen_buffer")
            .add_to(ctx);
        let write_driver_en_buffer = fanout_buffer_stage(2000e-15);
        ctx.instantiate::<LastBitDecoderStage>(&write_driver_en_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", write_driver_en),
                ("y_b", write_driver_en_b),
                ("predecode_0_0", write_driver_en0),
            ])
            .named("write_driver_en_buffer")
            .add_to(ctx);
        let sense_en_buffer = fanout_buffer_stage(800e-15);
        ctx.instantiate::<LastBitDecoderStage>(&sense_en_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", sense_en),
                ("y_b", sense_en_b),
                ("predecode_0_0", sense_en0),
            ])
            .named("sense_en_buffer")
            .add_to(ctx);

        let num_dffs = self.params.addr_width() + 2;
        ctx.instantiate::<DffArray>(&num_dffs)?
            .with_connections([("vdd", vdd), ("vss", vss), ("clk", clk), ("rb", reset_b)])
            .with_connection("d", Signal::new(vec![addr, we, ce]))
            .with_connection("q", Signal::new(vec![addr_in, we_in, ce_in]))
            .with_connection("qn", Signal::new(vec![addr_in_b, we_in_b, ce_in_b]))
            .named("addr_we_ce_dffs")
            .add_to(ctx);

        for i in 0..self.params.row_bits() {
            let buffer = fanout_buffer_stage(50e-15);
            ctx.instantiate::<LastBitDecoderStage>(&buffer)?
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("y", addr_gated.index(i)),
                    ("y_b", addr_gated_b.index(i)),
                    ("predecode_0_0", addr_gated0.index(i)),
                ])
                .named(format!("addr_gated_buffer_{i}"))
                .add_to(ctx);
            let buffer = fanout_buffer_stage(50e-15);
            ctx.instantiate::<LastBitDecoderStage>(&buffer)?
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("y", addr_b_gated.index(i)),
                    ("y_b", addr_b_gated_b.index(i)),
                    ("predecode_0_0", addr_b_gated0.index(i)),
                ])
                .named(format!("addr_b_gated_buffer_{i}"))
                .add_to(ctx);
        }

        ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows(),
            cols: self.params.cols(),
            mux_ratio: self.params.mux_ratio(),
        })?
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

        let replica_rows = ((self.params.rows() / 12) + 1) * 2;

        ctx.instantiate::<ReplicaCellArray>(&ReplicaCellArrayParams {
            rows: replica_rows,
            cols: 2,
        })?
        .with_connections([
            ("vdd", vdd),
            ("vss", vss),
            ("rbl", rbl),
            ("rbr", rbr),
            ("rwl", rwl),
        ])
        .named("replica_bitcell_array")
        .add_to(ctx);

        ctx.instantiate::<ColPeripherals>(&self.col_params())?
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

        for i in 0..2 {
            ctx.instantiate::<Precharge>(&self.col_params().pc)?
                .with_connections([("vdd", vdd), ("bl", rbl), ("br", rbr), ("en_b", pc_b0)])
                .named(format!("replica_precharge_{i}"))
                .add_to(ctx);
        }

        Ok(())
    }

    pub(crate) fn col_params(&self) -> ColParams {
        ColParams {
            pc: PrechargeParams {
                length: 150,
                pull_up_width: 2_000,
                equalizer_width: 1_200,
            },
            wrdriver: WriteDriverParams {
                length: 150,
                pwidth_driver: 10_000,
                nwidth_driver: 10_000,
                pwidth_logic: 3_000,
                nwidth_logic: 3_000,
            },
            mux: TGateMuxParams {
                length: 150,
                pwidth: 4_000,
                nwidth: 4_000,
                mux_ratio: self.params.mux_ratio(),
                idx: 0,
            },
            buf: PrimitiveGateParams {
                nwidth: 1_200,
                pwidth: 2_000,
                length: 150,
            },
            cols: self.params.cols(),
            wmask_granularity: self.params.cols()
                / self.params.mux_ratio()
                / self.params.wmask_width(),
            include_wmask: true,
        }
    }
}

pub(crate) fn buffer_chain_num_stages(cl: f64) -> usize {
    let fo = cl / INV_MODEL.cin;
    if fo < 2.0 {
        return 2;
    }
    let stages = 2 * (fo.log(3.0) / 2.0).round() as usize;
    let stages = if stages == 0 { 2 } else { stages };

    assert_eq!(stages % 2, 0);
    stages
}

pub(crate) fn inverter_chain_num_stages(cl: f64) -> usize {
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

pub(crate) fn fanout_buffer_stage(cl: f64) -> DecoderStageParams {
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

pub(crate) fn fanout_buffer_stage_with_inverted_output(cl: f64) -> DecoderStageParams {
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
