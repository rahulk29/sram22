use substrate::component::NoParams;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::signal::Signal;

use crate::blocks::bitcell_array::replica::ReplicaCellArray;
use crate::blocks::bitcell_array::SpCellArray;
use crate::blocks::columns::layout::DffArray;
use crate::blocks::columns::ColPeripherals;
use crate::blocks::control::ControlLogicReplicaV2;
use crate::blocks::decoder::{
    Decoder, DecoderPhysicalDesignParams, DecoderStage, DecoderStageParams, RoutingStyle, INV_MODEL,
};
use crate::blocks::gate::sizing::InverterGateTreeNode;
use crate::blocks::gate::GateParams;
use crate::blocks::precharge::Precharge;

use super::layout::{NeedsDiodes, ReplicaColumnMos};
use super::{SramInner, SramPhysicalDesignScript, TappedDiode};

impl SramInner {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<SramPhysicalDesignScript>(&self.params)?;
        let layout = ctx.inner().instantiate_layout::<SramInner>(&self.params)?;
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [clk, we, ce, rstb] = ctx.ports(["clk", "we", "ce", "rstb"], Direction::Input);

        let addr = ctx.bus_port("addr", self.params.addr_width(), Direction::Input);
        let wmask = ctx.bus_port("wmask", self.params.wmask_width(), Direction::Input);
        let din = ctx.bus_port("din", self.params.data_width(), Direction::Input);
        let dout = ctx.bus_port("dout", self.params.data_width(), Direction::Output);

        let [addr_in, addr_in_b] = ctx.buses(["addr_in", "addr_in_b"], self.params.addr_width());

        let [addr_gated, addr_b_gated] =
            ctx.buses(["addr_gated", "addr_b_gated"], self.params.row_bits());
        let addr_gate_y_b_noconn = ctx.bus("addr_gate_y_b_noconn", 2 * self.params.row_bits());

        let bl = ctx.bus("bl", self.params.cols());
        let br = ctx.bus("br", self.params.cols());
        let wl = ctx.bus("wl", self.params.rows());
        let wl_b = ctx.bus("wl_b", self.params.rows());

        let col_sel = ctx.bus("col_sel", self.params.mux_ratio());
        let col_sel_b = ctx.bus("col_sel_b", self.params.mux_ratio());

        // If needed, added antenna diodes to m1 pins.

        if let NeedsDiodes::Yes = layout.cell().get_metadata::<NeedsDiodes>() {
            for (port, width) in [
                // (dout, self.params.data_width()),
                (din, self.params.data_width()),
                (wmask, self.params.wmask_width()),
            ] {
                for i in 0..width {
                    ctx.instantiate::<TappedDiode>(&NoParams)?
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
        }

        let [we_in, we_in_b, ce_in, ce_in_b, rwl, rbl, rbr, pc_b0, pc, pc_b, wl_en0, wl_en_b, wl_en, write_driver_en0, write_driver_en_b, write_driver_en, sense_en0, sense_en_b, sense_en] =
            ctx.signals([
                "we_in",
                "we_in_b",
                "ce_in",
                "ce_in_b",
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

        ctx.instantiate::<DecoderStage>(&dsn.addr_gate)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("wl_en", wl_en),
                ("y_b", addr_gate_y_b_noconn),
            ])
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

        let mut row_decoder = ctx
            .instantiate::<Decoder>(&dsn.row_decoder)?
            .with_connections([("vdd", vdd), ("vss", vss), ("y", wl), ("y_b", wl_b)])
            .named("decoder");
        for i in 0..self.params.row_bits() {
            for j in 0..2 {
                row_decoder.connect(
                    format!("predecode_{i}_{j}"),
                    if j == 0 {
                        addr_b_gated.index(i)
                    } else {
                        addr_gated.index(i)
                    },
                );
            }
        }
        ctx.add_instance(row_decoder);

        let mut col_decoder = ctx
            .instantiate::<Decoder>(&dsn.col_decoder)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", col_sel),
                ("y_b", col_sel_b),
            ])
            .named("column_decoder");
        for i in 0..self.params.col_select_bits() {
            for j in 0..2 {
                col_decoder.connect(
                    format!("predecode_{i}_{j}"),
                    if j == 0 {
                        addr_in_b.index(i)
                    } else {
                        addr_in.index(i)
                    },
                );
            }
        }
        ctx.add_instance(col_decoder);

        let control_logic = ctx
            .instantiate::<ControlLogicReplicaV2>(&dsn.control)?
            .with_connections([
                ("clk", clk),
                ("we", we_in),
                ("ce", ce_in),
                ("rstb", rstb),
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

        ctx.instantiate::<DecoderStage>(&dsn.pc_b_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", pc_b),
                ("y_b", pc),
                ("predecode_0_0", pc_b0),
            ])
            .named("pc_b_buffer")
            .add_to(ctx);

        ctx.instantiate::<DecoderStage>(&dsn.wlen_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", wl_en),
                ("y_b", wl_en_b),
                ("predecode_0_0", wl_en0),
            ])
            .named("wlen_buffer")
            .add_to(ctx);

        ctx.instantiate::<DecoderStage>(&dsn.write_driver_en_buffer)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("y", write_driver_en),
                ("y_b", write_driver_en_b),
                ("predecode_0_0", write_driver_en0),
            ])
            .named("write_driver_en_buffer")
            .add_to(ctx);
        ctx.instantiate::<DecoderStage>(&dsn.sense_en_buffer)?
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
            .with_connections([("vdd", vdd), ("vss", vss), ("clk", clk), ("rb", rstb)])
            .with_connection("d", Signal::new(vec![addr, we, ce]))
            .with_connection("q", Signal::new(vec![addr_in, we_in, ce_in]))
            .with_connection("qn", Signal::new(vec![addr_in_b, we_in_b, ce_in_b]))
            .named("addr_we_ce_dffs")
            .add_to(ctx);

        ctx.instantiate::<SpCellArray>(&dsn.bitcells)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("dummy_bl", vdd),
                ("dummy_br", vdd),
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
                ("rstb", rstb),
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
            ctx.instantiate::<Precharge>(&dsn.replica_pc.inner)?
                .with_connections([("vdd", vdd), ("bl", rbl), ("br", rbr), ("en_b", pc_b0)])
                .named(format!("replica_precharge_{i}"))
                .add_to(ctx);
        }
        ctx.instantiate::<ReplicaColumnMos>(&dsn.replica_nmos)?
            .with_connections([("vdd", vdd), ("vss", vss), ("bl", rbl)])
            .named("replica_mos")
            .add_to(ctx);

        Ok(())
    }
}

pub(crate) fn buffer_chain_num_stages(cl: f64) -> usize {
    let fo = cl / INV_MODEL.cin;
    if fo < 4.0 {
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

pub fn fanout_buffer_stage(pd: DecoderPhysicalDesignParams, cl: f64) -> DecoderStageParams {
    let stages = buffer_chain_num_stages(cl);
    let invs = InverterGateTreeNode::buffer(stages)
        .elaborate()
        .size(cl)
        .as_inv_chain();
    DecoderStageParams {
        pd,
        routing_style: RoutingStyle::Decoder,
        max_width: None,
        gate: GateParams::FoldedInv(invs[0]),
        invs: invs.into_iter().skip(1).collect(),
        num: 1,
        use_multi_finger_invs: true,
        dont_connect_outputs: false,
        child_sizes: vec![1],
    }
}

pub fn fanout_buffer_stage_with_inverted_output(
    pd: DecoderPhysicalDesignParams,
    cl: f64,
) -> DecoderStageParams {
    let stages = inverter_chain_num_stages(cl);
    let mut invs = InverterGateTreeNode::inverter(stages)
        .elaborate()
        .size(cl)
        .as_inv_chain();
    invs.push(*invs.last().unwrap());
    DecoderStageParams {
        pd,
        routing_style: RoutingStyle::Decoder,
        max_width: None,
        gate: GateParams::FoldedInv(invs[0]),
        invs: invs.into_iter().skip(1).collect(),
        num: 1,
        use_multi_finger_invs: true,
        dont_connect_outputs: false,
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
