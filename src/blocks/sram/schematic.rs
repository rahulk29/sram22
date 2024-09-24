use substrate::component::NoParams;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::signal::Signal;

use crate::blocks::bitcell_array::replica::{ReplicaCellArray, ReplicaCellArrayParams};
use crate::blocks::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::blocks::buf::DiffBufParams;
use crate::blocks::columns::{ColParams, ColPeripherals};
use crate::blocks::control::{ControlLogicReplicaV2, DffArray, InvChain};
use crate::blocks::decoder::{
    AddrGate, AddrGateParams, Decoder, DecoderParams, DecoderStageParams, DecoderTree, WmuxDriver,
};
use crate::blocks::precharge::{Precharge, PrechargeParams};
use crate::blocks::rmux::ReadMuxParams;
use crate::blocks::wmux::WriteMuxSizing;

use super::{SramInner, READ_MUX_INPUT_CAP, WORDLINE_CAP_PER_CELL};

impl SramInner {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [clk, we, ce, reset] = ctx.ports(["clk", "we", "ce", "reset"], Direction::Input);

        let addr = ctx.bus_port("addr", self.params.addr_width, Direction::Input);
        let wmask = ctx.bus_port("wmask", self.params.wmask_width, Direction::Input);
        let din = ctx.bus_port("din", self.params.data_width, Direction::Input);
        let dout = ctx.bus_port("dout", self.params.data_width, Direction::Output);

        let [addr_in0, addr_in, addr_in0_b, addr_in_b] = ctx.buses(
            ["addr_in0", "addr_in", "addr_in0_b", "addr_in_b"],
            self.params.addr_width,
        );

        let [addr_gated, addr_b_gated] =
            ctx.buses(["addr_gated", "addr_b_gated"], self.params.row_bits);

        let bl = ctx.bus("bl", self.params.cols);
        let br = ctx.bus("br", self.params.cols);
        let wl = ctx.bus("wl", self.params.rows);
        let wl_b = ctx.bus("wl_b", self.params.rows);

        let col_sel = ctx.bus("col_sel", self.params.mux_ratio);
        let col_sel0_b = ctx.bus("col_sel0_b", self.params.mux_ratio);
        let col_sel_b = ctx.bus("col_sel_b", self.params.mux_ratio);

        let wmux_sel = ctx.bus("wmux_sel", self.params.mux_ratio);
        let wmux_sel_b = ctx.bus("wmux_sel_b", self.params.mux_ratio);

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;

        let diode = lib.try_cell_named("sky130_fd_sc_hd__diode_2")?;
        let bufbuf = lib.try_cell_named("sky130_fd_sc_hd__bufbuf_16")?;

        for (port, width) in [
            (dout, self.params.data_width),
            (din, self.params.data_width),
            (wmask, self.params.wmask_width),
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

        let [we_in, we_in_b, ce_in, ce_in_b, dummy_bl, dummy_br, rbl, rbr, pc_b0, pc_b, wl_en0, wl_en, write_driver_en0, write_driver_en, sense_en0, sense_en] =
            ctx.signals([
                "we_in",
                "we_in_b",
                "ce_in",
                "ce_in_b",
                "dummy_bl",
                "dummy_br",
                "rbl",
                "rbr",
                "pc_b0",
                "pc_b",
                "wl_en0",
                "wl_en",
                "write_driver_en0",
                "write_driver_en",
                "sense_en0",
                "sense_en",
            ]);
        let [decrepstart, decrepend] = ctx.signals(["decrepstart", "decrepend"]);

        let wl_cap = (self.params.cols + 4) as f64 * WORDLINE_CAP_PER_CELL;
        println!("wl_cap = {:.3}pF", wl_cap * 1e12);
        let tree = DecoderTree::new(self.params.row_bits, wl_cap);

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

        // TODO add wmux driver input capacitance
        let col_tree = DecoderTree::new(
            self.params.col_select_bits,
            READ_MUX_INPUT_CAP * (self.params.cols / self.params.mux_ratio) as f64,
        );
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
                ("decode_b", col_sel0_b),
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
        let control_logic = ctx
            .instantiate::<ControlLogicReplicaV2>(&NoParams)?
            .with_connections([
                ("clk", clk),
                ("we", we_in),
                ("ce", ce_in),
                ("reset", reset),
                ("rbl", rbl),
                ("pc_b", pc_b0),
                ("wlen", wl_en0),
                ("wrdrven", write_driver_en0),
                ("decrepstart", decrepstart),
                ("decrepend", decrepend),
                ("saen", sense_en0),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("control_logic");
        control_logic.add_to(ctx);
        ctx.instantiate::<InvChain>(&8)?
            .with_connections([
                ("din", decrepstart),
                ("dout", decrepend),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("decoder_replica")
            .add_to(ctx);

        for i in 0..self.params.mux_ratio {
            for _ in 0..3 {
                ctx.instantiate::<StdCell>(&bufbuf.id())?
                    .with_connections([
                        ("A", col_sel0_b.index(i)),
                        ("X", col_sel_b.index(i)),
                        ("VPWR", vdd),
                        ("VPB", vdd),
                        ("VGND", vss),
                        ("VNB", vss),
                    ])
                    .add_to(ctx);
            }
        }

        for _ in 0..5 {
            ctx.instantiate::<StdCell>(&bufbuf.id())?
                .with_connections([
                    ("A", pc_b0),
                    ("X", pc_b),
                    ("VPWR", vdd),
                    ("VPB", vdd),
                    ("VGND", vss),
                    ("VNB", vss),
                ])
                .add_to(ctx);
        }

        for _ in 0..3 {
            ctx.instantiate::<StdCell>(&bufbuf.id())?
                .with_connections([
                    ("A", wl_en0),
                    ("X", wl_en),
                    ("VPWR", vdd),
                    ("VPB", vdd),
                    ("VGND", vss),
                    ("VNB", vss),
                ])
                .add_to(ctx);
        }

        for _ in 0..3 {
            ctx.instantiate::<StdCell>(&bufbuf.id())?
                .with_connections([
                    ("A", write_driver_en0),
                    ("X", write_driver_en),
                    ("VPWR", vdd),
                    ("VPB", vdd),
                    ("VGND", vss),
                    ("VNB", vss),
                ])
                .add_to(ctx);
        }

        for _ in 0..3 {
            ctx.instantiate::<StdCell>(&bufbuf.id())?
                .with_connections([
                    ("A", sense_en0),
                    ("X", sense_en),
                    ("VPWR", vdd),
                    ("VPB", vdd),
                    ("VGND", vss),
                    ("VNB", vss),
                ])
                .add_to(ctx);
        }

        let num_dffs = self.params.addr_width + 2;
        ctx.instantiate::<DffArray>(&num_dffs)?
            .with_connections([("vdd", vdd), ("vss", vss), ("clk", clk)])
            .with_connection("d", Signal::new(vec![addr, we, ce]))
            .with_connection("q", Signal::new(vec![addr_in0, we_in, ce_in]))
            .with_connection("qn", Signal::new(vec![addr_in0_b, we_in_b, ce_in_b]))
            .named("addr_we_ce_dffs")
            .add_to(ctx);

        for i in 0..self.params.addr_width {
            for _ in 0..3 {
                ctx.instantiate::<StdCell>(&bufbuf.id())?
                    .with_connections([
                        ("A", addr_in0.index(i)),
                        ("X", addr_in.index(i)),
                        ("VPWR", vdd),
                        ("VPB", vdd),
                        ("VGND", vss),
                        ("VNB", vss),
                    ])
                    .add_to(ctx);
                ctx.instantiate::<StdCell>(&bufbuf.id())?
                    .with_connections([
                        ("A", addr_in0_b.index(i)),
                        ("X", addr_in_b.index(i)),
                        ("VPWR", vdd),
                        ("VPB", vdd),
                        ("VGND", vss),
                        ("VNB", vss),
                    ])
                    .add_to(ctx);
            }
        }

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
                ("dummy_bl", dummy_bl),
                ("dummy_br", dummy_br),
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
            rmux: ReadMuxParams {
                length: 150,
                width: 4_000,
                mux_ratio: self.params.mux_ratio,
                idx: 0,
            },
            wmux: WriteMuxSizing {
                length: 150,
                mux_width: 8_800,
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
