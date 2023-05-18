use substrate::component::NoParams;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::signal::Signal;

use crate::v2::bitcell_array::replica::{ReplicaCellArray, ReplicaCellArrayParams};
use crate::v2::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::v2::buf::DiffBufParams;
use crate::v2::columns::{ColParams, ColPeripherals};
use crate::v2::control::{ControlLogicReplicaV2, DffArray};
use crate::v2::decoder::{
    AddrGate, AddrGateParams, Decoder, DecoderParams, DecoderStageParams, DecoderTree, WmuxDriver,
};
use crate::v2::precharge::{Precharge, PrechargeParams};
use crate::v2::rmux::ReadMuxParams;
use crate::v2::wmux::WriteMuxSizing;

use super::SramInner;

impl SramInner {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [clk, we] = ctx.ports(["clk", "we"], Direction::Input);

        let addr = ctx.bus_port("addr", self.params.addr_width, Direction::Input);
        let wmask = ctx.bus_port("wmask", self.params.wmask_width, Direction::Input);
        let din = ctx.bus_port("din", self.params.data_width, Direction::Input);
        let dout = ctx.bus_port("dout", self.params.data_width, Direction::Output);

        let [addr_in, addr_in_b] = ctx.buses(["addr_in", "addr_in_b"], self.params.addr_width);

        let [addr_gated, addr_b_gated] =
            ctx.buses(["addr_gated", "addr_b_gated"], self.params.row_bits);

        let bl = ctx.bus("bl", self.params.cols);
        let br = ctx.bus("br", self.params.cols);
        let wl = ctx.bus("wl", self.params.rows);
        let wl_b = ctx.bus("wl_b", self.params.rows);

        let col_sel = ctx.bus("col_sel", self.params.mux_ratio);
        let col_sel_b = ctx.bus("col_sel_b", self.params.mux_ratio);

        let wmux_sel = ctx.bus("wmux_sel", self.params.mux_ratio);
        let wmux_sel_b = ctx.bus("wmux_sel_b", self.params.mux_ratio);

        let [we_in, we_in_b, dummy_bl, dummy_br, rbl, rbr, pc_b, wl_en0, wl_en, write_driver_en, sense_en] =
            ctx.signals([
                "we_in",
                "we_in_b",
                "dummy_bl",
                "dummy_br",
                "rbl",
                "rbr",
                "pc_b",
                "wl_en0",
                "wl_en",
                "write_driver_en",
                "sense_en",
            ]);

        let tree = DecoderTree::with_scale_and_skew(self.params.row_bits, 2, true);

        ctx.instantiate::<AddrGate>(&AddrGateParams {
            gate: tree.root.gate,
            num: self.params.row_bits,
        })?
        .with_connections([
            ("vdd", vdd),
            ("vss", vss),
            ("addr", addr_in.index(self.params.col_select_bits..)),
            ("addr_b", addr_in_b.index(self.params.col_select_bits..)),
            ("addr_gated", addr_gated),
            ("addr_b_gated", addr_b_gated),
            ("en", wl_en),
        ])
        .named("addr_gate")
        .add_to(ctx);

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

        let col_tree = DecoderTree::with_scale(self.params.col_select_bits, 2);
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };
        let wmux_driver_params = DecoderStageParams {
            gate: col_tree.root.gate,
            num: col_tree.root.num,
            child_sizes: vec![],
        };

        ctx.instantiate::<Decoder>(&col_decoder_params)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("addr", addr_in.index(0..self.params.col_select_bits)),
                ("addr_b", addr_in_b.index(0..self.params.col_select_bits)),
                ("decode", col_sel),
                ("decode_b", col_sel_b),
            ])
            .named("column_decoder")
            .add_to(ctx);

        ctx.instantiate::<WmuxDriver>(&wmux_driver_params)?
            .with_connection("in", col_sel)
            .with_connection("en", write_driver_en)
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("decode", wmux_sel),
                ("decode_b", wmux_sel_b),
            ])
            .named("wmux_driver")
            .add_to(ctx);
        ctx.instantiate::<ControlLogicReplicaV2>(&NoParams)?
            .with_connections([
                ("clk", clk),
                ("we", we_in),
                ("rbl", rbl),
                ("dummy_bl", dummy_bl),
                ("pc_b", pc_b),
                ("wl_en0", wl_en0),
                ("wl_en", wl_en),
                ("write_driver_en", write_driver_en),
                ("sense_en", sense_en),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("control_logic")
            .add_to(ctx);

        let num_dffs = self.params.addr_width + 1;
        ctx.instantiate::<DffArray>(&num_dffs)?
            .with_connections([("vdd", vdd), ("vss", vss), ("clk", clk)])
            .with_connection("d", Signal::new(vec![addr, we]))
            .with_connection("q", Signal::new(vec![addr_in, we_in]))
            .with_connection("qn", Signal::new(vec![addr_in_b, we_in_b]))
            .named("addr_we_dffs")
            .add_to(ctx);

        ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows,
            cols: self.params.cols,
            mux_ratio: self.params.mux_ratio,
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

        let replica_rows = ((self.params.rows / 12) + 1) * 2;

        ctx.instantiate::<ReplicaCellArray>(&ReplicaCellArrayParams {
            rows: replica_rows,
            cols: 2,
        })?
        .with_connections([
            ("vdd", vdd),
            ("vss", vss),
            ("rbl", rbl),
            ("rbr", rbr),
            ("rwl", wl_en0),
        ])
        .named("replica_bitcell_array")
        .add_to(ctx);

        ctx.instantiate::<ColPeripherals>(&self.col_params())?
            .with_connections([
                ("clk", clk),
                ("vdd", vdd),
                ("vss", vss),
                ("bl", bl),
                ("br", br),
                ("pc_b", pc_b),
                ("sel_b", col_sel_b),
                ("we", wmux_sel),
                ("wmask", wmask),
                ("din", din),
                ("dout", dout),
                ("sense_en", sense_en),
            ])
            .named("col_circuitry")
            .add_to(ctx);

        ctx.instantiate::<Precharge>(&self.col_params().pc)?
            .with_connections([("vdd", vdd), ("bl", rbl), ("br", rbr), ("en_b", pc_b)])
            .named("replica_precharge")
            .add_to(ctx);

        ctx.instantiate::<Precharge>(&self.col_params().pc)?
            .with_connections([
                ("vdd", vdd),
                ("bl", dummy_bl),
                ("br", dummy_br),
                ("en_b", pc_b),
            ])
            .named("dummy_precharge")
            .add_to(ctx);

        Ok(())
    }

    pub(crate) fn col_params(&self) -> ColParams {
        ColParams {
            pc: PrechargeParams {
                length: 150,
                pull_up_width: 2_000,
                equalizer_width: 1_200,
            },
            rmux: ReadMuxParams {
                length: 150,
                width: 3_000,
                mux_ratio: self.params.mux_ratio,
                idx: 0,
            },
            wmux: WriteMuxSizing {
                length: 150,
                mux_width: 2_400,
                mux_ratio: self.params.mux_ratio,
            },
            buf: DiffBufParams {
                width: 4_800,
                nw: 1_200,
                pw: 2_000,
                lch: 150,
            },
            cols: self.params.cols,
            wmask_granularity: self.params.cols / self.params.mux_ratio / self.params.wmask_width,
            include_wmask: true,
        }
    }
}
