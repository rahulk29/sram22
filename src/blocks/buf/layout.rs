use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Rect, Span};
use substrate::component::{Component, NoParams};
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
use substrate::layout::placement::align::{AlignMode, AlignRect};

use crate::blocks::gate::{FoldedInv, PrimitiveGateParams};
use crate::blocks::macros::SenseAmp;

use super::DiffBuf;

pub const POWER_HEIGHT: i64 = 800;
pub const GRID: i64 = 5;
pub const WELL_PAD: i64 = 1_000;

impl DiffBuf {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;

        let blinv = ctx
            .instantiate::<FoldedInv>(&self.params)?
            .with_orientation(Named::R90Cw);
        let mut brinv = ctx
            .instantiate::<FoldedInv>(&self.params)?
            .with_orientation(Named::R90Cw);
        brinv.align(AlignMode::ToTheRight, &blinv, 170);
        brinv.align(AlignMode::CenterVertical, &blinv, 0);
        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.align(
            AlignMode::CenterHorizontal,
            blinv.bbox().union(brinv.bbox()),
            0,
        );
        let hspan = sa.brect().hspan();
        for (port, in_port, inst) in [("outp", "din1", &blinv), ("outn", "din2", &brinv)] {
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
            let power_span = Span::from_center_span_gridded(
                blinv.port(port)?.largest_rect(m0)?.center().y,
                POWER_HEIGHT,
                ctx.pdk().layout_grid(),
            );
            let power_stripe = Rect::from_spans(hspan, power_span);
            for inst in [&blinv, &brinv] {
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

        ctx.add_port(
            CellPort::builder()
                .id("dout1")
                .add(m0, brinv.port("y")?.largest_rect(m0)?)
                .build(),
        )?;
        ctx.add_port(
            CellPort::builder()
                .id("dout2")
                .add(m0, blinv.port("y")?.largest_rect(m0)?)
                .build(),
        )?;

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
        ctx.draw(blinv)?;
        ctx.draw(brinv)?;
        Ok(())
    }
}

pub struct DiffBufCent {
    params: PrimitiveGateParams,
}

impl Component for DiffBufCent {
    type Params = PrimitiveGateParams;
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

        let pc = ctx
            .inner()
            .run_script::<crate::blocks::precharge::layout::PhysicalDesignScript>(&NoParams)?;

        let buf = ctx.instantiate::<DiffBuf>(&self.params)?;
        let hspan = Span::new(0, pc.tap_width);
        let bounds = Rect::from_spans(hspan, buf.brect().vspan());

        ctx.draw_rect(
            nwell,
            Rect::from_spans(hspan, buf.layer_bbox(nwell).into_rect().vspan()),
        );

        let nspan = buf.layer_bbox(nsdm).into_rect().vspan();
        let pspan = buf.layer_bbox(psdm).into_rect().vspan();

        for (span, vdd) in [(pspan, true), (nspan, false)] {
            let r = Rect::from_spans(hspan, span).shrink(200);
            let viap = ViaParams::builder().layers(tap, m0).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
            let sdm_rect = via.layer_bbox(tap).into_rect().expand(130);
            ctx.draw_rect(if vdd { nsdm } else { psdm }, sdm_rect);

            let pspan = buf
                .port(if vdd { "vdd" } else { "vss" })?
                .largest_rect(m2)?
                .vspan();
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
