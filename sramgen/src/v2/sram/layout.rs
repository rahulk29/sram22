use substrate::component::NoParams;
use substrate::error::Result;
use substrate::layout::cell::Port;
use substrate::layout::context::LayoutCtx;
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::{Dir, Side};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::SJog;

use crate::bus_bit;
use crate::v2::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::v2::buf::DiffBufParams;
use crate::v2::columns::{ColParams, ColPeripherals};
use crate::v2::control::{ControlLogicReplicaV1, DffArray};
use crate::v2::decoder::layout::LastBitDecoderStage;
use crate::v2::decoder::{
    DecoderParams, DecoderStageParams, DecoderTree, Predecoder, WlDriver, WmuxDriver,
};
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
            wmask_granularity: 8,
            include_wmask: true,
        })?;
        let tree = DecoderTree::new(self.params.row_bits);
        let decoder_params = DecoderStageParams {
            gate: tree.root.gate,
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
            tree: DecoderTree {
                root: tree.root.children[0].clone(),
            },
        })?;
        let p1_bits = tree.root.children[0].num.ilog2() as usize;
        let p2_bits = tree.root.children[1].num.ilog2() as usize;

        let mut p2 = ctx.instantiate::<Predecoder>(&DecoderParams {
            tree: DecoderTree {
                root: tree.root.children[1].clone(),
            },
        })?;

        let col_tree = DecoderTree::new(self.params.col_select_bits);
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };
        let col_bits = col_tree.root.num.ilog2() as usize;

        let mut col_dec = ctx.instantiate::<Predecoder>(&col_decoder_params)?;
        let wmux_driver_params = DecoderStageParams {
            gate: col_tree.root.gate,
            num: col_tree.root.num,
            child_sizes: vec![],
        };
        let mut wmux_driver = ctx.instantiate::<WmuxDriver>(&wmux_driver_params)?;
        let mut control = ctx
            .instantiate::<ControlLogicReplicaV1>(&NoParams)?
            .with_orientation(Named::R90);

        let num_dffs = self.params.addr_width + 1;
        let mut dffs = ctx.instantiate::<DffArray>(&num_dffs)?;

        cols.align_beneath(bitcells.bbox(), 1_270);
        cols.align_centers_horizontally_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        wl_driver.align_to_the_left_of(bitcells.bbox(), 1_270);
        wl_driver.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_to_the_left_of(wl_driver.bbox(), 1_270);
        decoder.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_side_to_grid(Side::Left, 640);
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

        ctx.draw_ref(&bitcells)?;
        ctx.draw_ref(&cols)?;
        ctx.draw_ref(&decoder)?;
        ctx.draw_ref(&wl_driver)?;
        ctx.draw_ref(&wmux_driver)?;
        ctx.draw_ref(&p1)?;
        ctx.draw_ref(&p2)?;
        ctx.draw_ref(&col_dec)?;
        ctx.draw_ref(&control)?;
        ctx.draw_ref(&dffs)?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(5 * 680).snap_to_grid(680),
            layers: vec![
                LayerConfig {
                    line: 170,
                    space: 170,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 340,
                    space: 340,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 340,
                    space: 340,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        for inst in [&bitcells, &cols] {
            router.block(m2, inst.brect());
            router.block(m3, inst.brect());
        }
        for inst in [&p1, &p2, &col_dec, &control, &wmux_driver, &dffs] {
            for shape in inst.shapes_on(m2) {
                let rect = shape.brect();
                router.block(m2, rect);
            }
        }

        // Route address bits from DFFs to decoders
        let mut ctr = 0;
        for (inst, num) in [(&p1, p1_bits), (&p2, p2_bits), (&col_dec, col_bits)] {
            for i in 0..num {
                let src = dffs.port(&bus_bit("q", ctr))?.largest_rect(m2)?;
                let dst = inst.port(&format!("predecode_{i}_0"))?.largest_rect(m2)?;
                router.route(ctx, m2, src, m2, dst)?;
                let src = dffs.port(&bus_bit("qn", ctr))?.largest_rect(m2)?;
                let dst = inst.port(&format!("predecode_{i}_1"))?.largest_rect(m2)?;
                router.route(ctx, m2, src, m2, dst)?;
                ctr += 1;
            }
        }

        // Route predecoders to final decoder stage
        for i in 0..tree.root.children[0].num {
            let src = p1.port(&format!("decode_{i}"))?.largest_rect(m0)?;
            ctx.draw_rect(m1, src);
            let dst = decoder
                .port(&format!("predecode_0_{i}"))?
                .largest_rect(m1)?;
            router.route(ctx, m1, src, m1, dst)?;
        }
        for i in 0..tree.root.children[1].num {
            let src = p2.port(&format!("decode_{i}"))?.largest_rect(m0)?;
            let dst = decoder
                .port(&format!("predecode_1_{i}"))?
                .largest_rect(m1)?;
            router.route(ctx, m1, src, m1, dst)?;
        }

        // Route wordline decoder to wordlin driver
        for i in 0..tree.root.num {
            let src = decoder.port(&format!("decode_{i}"))?.largest_rect(m0)?;
            let dst = wl_driver.port(&bus_bit("in", i))?.largest_rect(m0)?;
            let jog = SJog::builder()
                .src(src)
                .dst(dst)
                .dir(Dir::Horiz)
                .layer(m0)
                .width(170)
                .grid(ctx.pdk().layout_grid())
                .build()
                .unwrap();
            ctx.draw(jog)?;
        }

        // Route column decoder to wmux driver
        for i in 0..col_tree.root.num {
            let src = col_dec.port(&format!("decode_{i}"))?.largest_rect(m0)?;
            let dst = wmux_driver.port(&bus_bit("in", i))?.largest_rect(m0)?;
            let jog = SJog::builder()
                .src(src)
                .dst(dst)
                .dir(Dir::Vert)
                .layer(m0)
                .width(170)
                .grid(ctx.pdk().layout_grid())
                .build()
                .unwrap();
            ctx.draw(jog)?;
        }

        ctx.draw(router)?;
        Ok(())
    }
}
