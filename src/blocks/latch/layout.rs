use crate::blocks::columns::ColumnDesignScript;
use crate::blocks::decoder::layout::{DecoderVia, DecoderViaParams};
use crate::blocks::gate::{FoldedInv, PrimitiveGateParams};
use crate::blocks::macros::SenseAmp;
use crate::blocks::sram::layout::draw_via;
use itertools::Itertools;
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Rect, Side, Span};
use substrate::component::{Component, NoParams};
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

use super::{DiffLatch, DiffLatchParams};

pub const POWER_HEIGHT: i64 = 800;
pub const GRID: i64 = 5;
pub const WELL_PAD: i64 = 1_000;

impl DiffLatch {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let db = ctx.mos_db();
        let nmos = db.default_nmos().unwrap();
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;

        let blinv_in = ctx
            .instantiate::<FoldedInv>(&self.params.inv_in)?
            .with_orientation(Named::R90Cw);
        let mut brinv_in = blinv_in.clone();
        brinv_in.align(AlignMode::ToTheRight, &blinv_in, 600);
        brinv_in.align(AlignMode::CenterVertical, &blinv_in, 0);
        let mut mn1 = ctx
            .instantiate::<LayoutMos>(&LayoutMosParams {
                skip_sd_metal: vec![vec![]; 3],
                deep_nwell: true,
                contact_strategy: GateContactStrategy::Merge,
                devices: vec![MosParams {
                    w: self.params.nwidth / 2,
                    l: self.params.lch,
                    m: 1,
                    nf: 2,
                    id: nmos.id(),
                }],
            })?
            .with_orientation(Named::R90Cw);
        let mut mn2 = mn1.clone();
        mn1.align(AlignMode::CenterHorizontal, &blinv_in, 0);
        mn1.align_beneath(&blinv_in, 170);
        mn2.align(AlignMode::CenterHorizontal, &brinv_in, 0);
        mn2.align_beneath(&brinv_in, 170);

        let mut blinvq = ctx
            .instantiate::<FoldedInv>(&self.params.invq)?
            .with_orientation(Named::R90Cw);
        let mut brinvq = blinvq.clone();
        blinvq.align(AlignMode::CenterHorizontal, &mn1, 0);
        blinvq.align_beneath(&mn1, 210);
        brinvq.align(AlignMode::CenterHorizontal, &mn2, 0);
        brinvq.align_beneath(&mn2, 210);

        let mut blinv_out = ctx
            .instantiate::<FoldedInv>(&self.params.inv_out)?
            .with_orientation(Named::R90Cw);
        let mut brinv_out = blinv_out.clone();
        blinv_out.align(AlignMode::CenterHorizontal, &blinvq, 0);
        blinv_out.align_beneath(&blinvq, 170);
        brinv_out.align(AlignMode::CenterHorizontal, &brinvq, 0);
        brinv_out.align_beneath(&brinvq, 170);

        for (inst1, port1, inst2, port2) in [
            (&blinv_in, "y", &mn1, "gate"),
            (&brinv_in, "y", &mn2, "gate"),
            (&mn1, "sd_0_1", &blinvq, "a"),
            (&mn2, "sd_0_1", &brinvq, "a"),
            (&blinvq, "y", &blinv_out, "a"),
            (&brinvq, "y", &brinv_out, "a"),
        ] {
            let rect1 = inst1.port(port1)?.largest_rect(m0)?;
            let rect2 = inst2.port(port2)?.largest_rect(m0)?;
            ctx.draw_rect(m0, rect1.with_vspan(rect1.vspan().union(rect2.vspan())));
        }

        let al = blinvq.port("a")?.largest_rect(m0)?;
        let ar = brinvq.port("a")?.largest_rect(m0)?;
        let m0_trackl = Span::with_start_and_length(al.bottom(), 170);
        let m0_trackr = m0_trackl.translate(340);
        let vss_rectl = blinvq.port("vss")?.first_rect(m0, Side::Right)?;
        let vss_rectr = brinvq.port("vss")?.first_rect(m0, Side::Left)?;
        let m1_trackl = Span::with_start_and_length(vss_rectl.right() + 200, 200);
        let m1_trackr = Span::with_stop_and_length(vss_rectr.left() - 200, 200);
        let m0_track_final = Span::with_stop_and_length(vss_rectl.bottom() - 170, 170);

        for (a, m0_track, m1_track, b) in [
            (al, m0_trackl, m1_trackr, ar),
            (ar, m0_trackr, m1_trackl, al),
        ] {
            let rect1 = Rect::from_spans(a.hspan().union(m1_track), m0_track);
            ctx.draw_rect(m0, rect1);
            let rect2 = Rect::from_spans(m1_track, m0_track.union(m0_track_final));
            ctx.draw_rect(m1, rect2);
            let rect3 = Rect::from_spans(b.hspan().union(m1_track), m0_track_final);
            ctx.draw_rect(m0, rect3);
            draw_via(m0, rect1, m1, rect2, ctx)?;
            draw_via(m0, rect3, m1, rect2, ctx)?;
        }

        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.align(
            AlignMode::CenterHorizontal,
            blinv_in.bbox().union(brinv_in.bbox()),
            0,
        );
        let hspan = sa.brect().hspan();
        for (port, in_port, inst) in [("outp", "din1", &blinv_in), ("outn", "din2", &brinv_in)] {
            let sa_out = sa.port(port)?.largest_rect(m1)?;
            let inv_in = inst.port("a")?.bbox(m0).into_rect();

            let m0_rect = Rect::from_spans(inv_in.hspan().union(sa_out.hspan()), inv_in.vspan());
            let m1_rect = Rect::from_spans(
                sa_out.hspan(),
                inv_in.vspan().add_point(inst.brect().top() + 330),
            );
            ctx.draw_rect(m0, m0_rect);
            ctx.draw_rect(m1, m1_rect);
            ctx.add_port(CellPort::builder().id(in_port).add(m1, m1_rect).build())?;
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(m0_rect, m1_rect)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
        }

        for port in ["vdd", "vss"] {
            for (blinv, brinv) in [
                (&blinv_in, &brinv_in),
                (&blinvq, &brinvq),
                (&blinv_out, &brinv_out),
            ] {
                let power_span = Span::from_center_span_gridded(
                    blinv.port(port)?.largest_rect(m0)?.center().y,
                    POWER_HEIGHT,
                    ctx.pdk().layout_grid(),
                );
                let power_stripe = Rect::from_spans(hspan, power_span);
                for inst in [blinv, brinv] {
                    for pwr in inst
                        .port(port)?
                        .shapes(m0)
                        .filter_map(|shape| shape.as_rect())
                    {
                        let viap = ViaParams::builder()
                            .layers(m0, m1)
                            .geometry(pwr, pwr)
                            .expand(ViaExpansion::LongerDirection)
                            .build();
                        let via = ctx.instantiate::<Via>(&viap)?;
                        ctx.draw_ref(&via)?;
                        let viap = ViaParams::builder()
                            .layers(m1, m2)
                            .geometry(via.layer_bbox(m1), power_stripe)
                            .expand(ViaExpansion::LongerDirection)
                            .build();
                        let via = ctx.instantiate::<Via>(&viap)?;
                        ctx.draw(via)?;
                    }
                }
                ctx.draw_rect(m2, power_stripe);
                ctx.merge_port(CellPort::with_shape(port, m2, power_stripe));
            }
        }

        let power_span = Span::from_center_span_gridded(
            mn1.port("sd_0_0")?.largest_rect(m0)?.center().y,
            POWER_HEIGHT,
            ctx.pdk().layout_grid(),
        );
        let power_stripe = Rect::from_spans(hspan, power_span);
        ctx.draw_rect(m2, power_stripe);
        ctx.merge_port(CellPort::with_shape("vss", m2, power_stripe));
        for port in ["sd_0_0", "sd_0_2"] {
            for inst in [&mn1, &mn2] {
                let rect = inst.port(port)?.largest_rect(m0)?;
                let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                    rect: rect.with_vspan(power_span),
                    via_metals: vec![m0, m1, m2],
                })?;
                ctx.draw(via)?;
            }
        }

        ctx.add_port(
            CellPort::builder()
                .id("dout1")
                .add(m0, blinv_out.port("y")?.largest_rect(m0)?)
                .build(),
        )?;
        ctx.add_port(
            CellPort::builder()
                .id("dout2")
                .add(m0, brinv_out.port("y")?.largest_rect(m0)?)
                .build(),
        )?;

        for (blinv, brinv) in [
            (&blinv_in, &brinv_in),
            (&blinvq, &brinvq),
            (&blinv_out, &brinv_out),
        ] {
            ctx.draw_rect(nwell, blinv.layer_bbox(nwell).into_rect().with_hspan(hspan));
            ctx.draw_rect(
                nsdm,
                blinv
                    .layer_bbox(nsdm)
                    .union(brinv.layer_bbox(nsdm))
                    .into_rect(),
            );
            ctx.draw_rect(
                psdm,
                blinv
                    .layer_bbox(psdm)
                    .union(brinv.layer_bbox(psdm))
                    .into_rect(),
            );
        }

        for inst in [
            blinv_in, brinv_in, mn1, mn2, blinvq, brinvq, blinv_out, brinv_out,
        ] {
            ctx.draw(inst)?;
        }
        Ok(())
    }
}

pub struct DiffLatchCent {
    params: DiffLatchParams,
}

impl Component for DiffLatchCent {
    type Params = DiffLatchParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("diff_buf_cent")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let outline = layers.get(Selector::Name("outline"))?;
        let tap = layers.get(Selector::Name("tap"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let buf = ctx.instantiate::<DiffLatch>(&self.params)?;
        let hspan = Span::new(0, pc.tap_width);
        let bounds = Rect::from_spans(hspan, buf.brect().vspan());

        for nwell_span in Span::merge_adjacent(
            buf.shapes_on(nwell)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| rect.vspan()),
            |a, b| a.intersects(&b),
        ) {
            ctx.draw_rect(nwell, Rect::from_spans(hspan, nwell_span));
        }

        let nspans = Span::merge_adjacent(
            buf.shapes_on(nsdm)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| rect.vspan()),
            |a, b| a.intersects(&b),
        )
        .sorted_by(|a, b| a.start().cmp(&b.start()));
        let pspans = Span::merge_adjacent(
            buf.shapes_on(psdm)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| rect.vspan()),
            |a, b| a.intersects(&b),
        )
        .sorted_by(|a, b| a.start().cmp(&b.start()));
        let vdd_spans = Span::merge_adjacent(
            buf.port("vdd")?
                .shapes(m2)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| rect.vspan()),
            |a, b| a.intersects(&b),
        )
        .sorted_by(|a, b| a.start().cmp(&b.start()));
        let vss_spans = Span::merge_adjacent(
            buf.port("vss")?
                .shapes(m2)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| rect.vspan()),
            |a, b| a.intersects(&b),
        )
        .sorted_by(|a, b| a.start().cmp(&b.start()));

        for (span, pspan, vdd) in nspans
            .zip(vss_spans)
            .map(|(span, pspan)| (span, pspan, false))
            .chain(
                pspans
                    .zip(vdd_spans)
                    .map(|(span, pspan)| (span, pspan, true)),
            )
        {
            let r = Rect::from_spans(hspan, span).shrink(200);
            let viap = ViaParams::builder().layers(tap, m0).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
            let sdm_rect = via.layer_bbox(tap).into_rect().expand(130);
            ctx.draw_rect(if vdd { nsdm } else { psdm }, sdm_rect);

            let power_stripe = Rect::from_spans(hspan, pspan);

            let viap = ViaParams::builder().layers(m0, m1).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;

            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(via.layer_bbox(m1), power_stripe)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            ctx.draw_rect(m2, power_stripe);

            let name = if vdd {
                arcstr::literal!("vdd")
            } else {
                arcstr::literal!("vss")
            };
            ctx.merge_port(CellPort::with_shape(name, m2, power_stripe));
        }
        ctx.draw_rect(outline, bounds);

        Ok(())
    }
}
