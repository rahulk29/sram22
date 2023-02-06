use substrate::component::NoParams;
use substrate::error::Result;
use substrate::layout::context::LayoutCtx;
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::orientation::Named;
use substrate::layout::placement::align::AlignRect;

use crate::v2::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::v2::buf::DiffBufParams;
use crate::v2::columns::{ColParams, ColPeripherals};
use crate::v2::control::{ControlLogicReplicaV1, DffArray};
use crate::v2::decoder::layout::LastBitDecoderStage;
use crate::v2::decoder::{DecoderStageParams, DecoderTree, WlDriver, DecoderParams, Predecoder, WmuxDriver};
use crate::v2::precharge::PrechargeParams;
use crate::v2::rmux::ReadMuxParams;
use crate::v2::wmux::WriteMuxSizing;

use super::Sram;

impl Sram {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let bitcells = ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows,
            cols: self.params.cols,
            mux_ratio: self.params.mux_ratio,
        })?;
        let mut cols = ctx.instantiate::<ColPeripherals>(&ColParams {
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
            mask_granularity: 8, // TODO
        })?;
        let tree = DecoderTree::new(self.params.row_bits);
        let decoder_params = DecoderStageParams {
            gate: tree.root.gate.clone(),
            num: tree.root.num,
            child_sizes: tree.root.children.iter().map(|n| n.num).collect(),
        };
        let mut decoder = ctx
            .instantiate::<LastBitDecoderStage>(&decoder_params)?
            .with_orientation(Named::R90Cw);
        let mut wl_driver = ctx
            .instantiate::<WlDriver>(&decoder_params)?
            .with_orientation(Named::R90Cw);

        let mut p1 = ctx.instantiate::<Predecoder>(&DecoderParams {
            tree: DecoderTree { root: tree.root.children[0].clone()},
        })?;

        let mut p2 = ctx.instantiate::<Predecoder>(&DecoderParams {
            tree: DecoderTree { root: tree.root.children[1].clone()},
        })?;

        let col_tree = DecoderTree::new(self.params.col_select_bits);
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };
        let mut col_dec = ctx.instantiate::<Predecoder>(&col_decoder_params)?;
        let wmux_driver_params = DecoderStageParams {
            gate: col_tree.root.gate.clone(),
            num: col_tree.root.num,
            child_sizes: vec![],
        };
        let mut wmux_driver = ctx
            .instantiate::<WmuxDriver>(&wmux_driver_params)?;
        let mut control = ctx.instantiate::<ControlLogicReplicaV1>(&NoParams)?.with_orientation(Named::R90);

        let num_dffs = self.params.addr_width + 1;
        let mut dffs = ctx.instantiate::<DffArray>(&num_dffs)?;

        cols.align_beneath(bitcells.bbox(), 1_270);
        cols.align_centers_horizontally_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        wl_driver.align_to_the_left_of(bitcells.bbox(), 1_270);
        wl_driver.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_to_the_left_of(wl_driver.bbox(), 1_270);
        decoder.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        p1.align_beneath(wl_driver.bbox(), 1_270);
        p1.align_right(wl_driver.bbox());
        p2.align_beneath(p1.bbox(), 1_270);
        p2.align_right(wl_driver.bbox());
        wmux_driver.align_beneath(p2.bbox(), 1_270);
        wmux_driver.align_right(wl_driver.bbox());
        col_dec.align_beneath(wmux_driver.bbox(), 1_270);
        col_dec.align_right(wl_driver.bbox());
        control.align_beneath(col_dec.bbox(), 1_270);
        control.align_right(wl_driver.bbox());
        dffs.align_beneath(control.bbox(), 1_270);
        dffs.align_right(wl_driver.bbox());

        ctx.draw(bitcells)?;
        ctx.draw(cols)?;
        ctx.draw(decoder)?;
        ctx.draw(wl_driver)?;
        ctx.draw(wmux_driver)?;
        ctx.draw(p1)?;
        ctx.draw(p2)?;
        ctx.draw(col_dec)?;
        ctx.draw(control)?;
        ctx.draw(dffs)?;
        Ok(())
    }
}
