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
use crate::v2::control::{ControlLogicReplicaV1, DffArray};
use crate::v2::decoder::{
    Decoder, DecoderParams, DecoderStageParams, DecoderTree, WlDriver, WmuxDriver,
};
use crate::v2::precharge::PrechargeParams;
use crate::v2::rmux::ReadMuxParams;
use crate::v2::wmux::WriteMuxSizing;

use super::Sram;

impl Sram {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [clk, we] = ctx.ports(["clk", "we"], Direction::Input);

        let addr = ctx.bus_port("addr", self.params.addr_width, Direction::Input);
        let addr_in = ctx.bus("addr_in", self.params.addr_width);
        let addr_in_b = ctx.bus("addr_in_b", self.params.addr_width);

        let addr_decode = ctx.bus("addr_decode", self.params.rows);
        let addr_decode_b = ctx.bus("addr_decode_b", self.params.rows);

        let col_sel = ctx.bus("col_sel", self.params.mux_ratio);
        let col_sel_b = ctx.bus("col_sel_b", self.params.mux_ratio);

        let wmux_sel = ctx.bus("wmux_sel", self.params.mux_ratio);
        let wmux_sel_b = ctx.bus("wmux_sel_b", self.params.mux_ratio);

        let tree = DecoderTree::new(self.params.row_bits);

        let driver_params = DecoderStageParams {
            gate: tree.root.gate,
            num: tree.root.num,
            child_sizes: tree.root.children.iter().map(|n| n.num).collect(),
        };

        let decoder_params = DecoderParams { tree };

        let decoder = ctx
            .instantiate::<Decoder>(&decoder_params)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("addr", addr_in.index(self.params.col_select_bits..)),
                ("addr_b", addr_in_b.index(self.params.col_select_bits..)),
                ("decode", addr_decode),
                ("decode_b", addr_decode_b),
            ])
            .named("decoder");
        ctx.add_instance(decoder);

        let wl_driver = ctx.instantiate::<WlDriver>(&driver_params)?;
        ctx.add_instance(wl_driver);

        let col_tree = DecoderTree::new(self.params.col_select_bits);
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };
        let wmux_driver_params = DecoderStageParams {
            gate: col_tree.root.gate,
            num: col_tree.root.num,
            child_sizes: vec![],
        };

        let col_dec = ctx
            .instantiate::<Decoder>(&col_decoder_params)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("addr", addr_in.index(0..self.params.col_select_bits)),
                ("addr_b", addr_in_b.index(0..self.params.col_select_bits)),
            ])
            .add_to(ctx);
        let mut wmux_driver = ctx
            .instantiate::<WmuxDriver>(&wmux_driver_params)?
            .with_connection("a", col_sel)
            .with_connection("b", Signal::repeat(we, self.params.mux_ratio))
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("decode", wmux_sel),
                ("decode_b", wmux_sel_b),
            ])
            .named("wmux_driver")
            .add_to(ctx);
        let mut control = ctx.instantiate::<ControlLogicReplicaV1>(&NoParams)?;

        let num_dffs = self.params.addr_width + 1;
        let mut dffs = ctx.instantiate::<DffArray>(&num_dffs)?;

        let bitcells = ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows,
            cols: self.params.cols,
            mux_ratio: self.params.mux_ratio,
        })?;

        let replica_rows = (self.params.rows / 12) * 2;

        let replica = ctx.instantiate::<ReplicaCellArray>(&ReplicaCellArrayParams {
            rows: replica_rows,
            cols: 2,
        })?;

        let mut cols = ctx.instantiate::<ColPeripherals>(&self.col_params())?;

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
            wmask_granularity: 8,
            include_wmask: true,
        }
    }
}
