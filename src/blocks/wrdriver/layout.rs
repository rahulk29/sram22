use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Dir, Rect, Span};
use substrate::component::{Component, NoParams};
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;

use substrate::layout::placement::align::{AlignMode, AlignRect};
use crate::blocks::columns::ColumnDesignScript;
use crate::blocks::delay_line::tristate::TristateInv;
use crate::blocks::gate::PrimitiveGateParams;
use crate::blocks::macros::SenseAmp;

use super::{WriteDriver, WriteDriverParams};

pub const POWER_HEIGHT: i64 = 800;

impl WriteDriver {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;

        let driver_params = PrimitiveGateParams {
            pwidth: self.params.pwidth_driver,
            nwidth: self.params.nwidth_driver,
            length: self.params.length,
        };

        let blinv = ctx
            .instantiate::<TristateInv>(&driver_params)?
            .with_orientation(Named::R90);
        let mut brinv = ctx
            .instantiate::<TristateInv>(&driver_params)?
            .with_orientation(Named::FlipYx);
        brinv.align(AlignMode::ToTheRight, &blinv, 170);
        brinv.align(AlignMode::CenterVertical, &blinv, 0);
        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.align(
            AlignMode::CenterHorizontal,
            blinv.bbox().union(brinv.bbox()),
            0,
        );
        let hspan = sa.brect().hspan();
        for (port, out_port, inst) in [("inp", "bl", &blinv), ("inn", "br", &brinv)] {
            let sa_in = sa.port(port)?.largest_rect(m1)?;
            let inv_out = inst.port("dout")?.largest_rect(m0)?;
            let inv_out_vspan = Span::with_stop_and_length(inv_out.top(), 170);

            let m0_rect = Rect::from_spans(inv_out.hspan().union(sa_in.hspan()), inv_out_vspan);
            let m1_rect =
                Rect::from_spans(sa_in.hspan(), inv_out_vspan.add_point(inst.brect().top()));
            ctx.draw_rect(m0, m0_rect);
            ctx.draw_rect(m1, m1_rect);
            ctx.add_port(CellPort::builder().id(out_port).add(m1, m1_rect).build())?;
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(m0_rect, m1_rect)
                .top_extension(Dir::Vert)
                .bot_extension(Dir::Horiz)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
        }

        for port in ["en", "en_b"] {
            let mut vias = Vec::new();

            for inst in [&blinv, &brinv] {
                let port_rect = inst.port(port)?.largest_rect(m0)?;
                let viap = ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(port_rect, port_rect)
                    .expand(ViaExpansion::LongerDirection)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw_ref(&via)?;
                let viap = ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(port_rect, port_rect)
                    .expand(ViaExpansion::LongerDirection)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw_ref(&via)?;
                vias.push(via);
            }

            let port_rect = vias
                .iter()
                .map(|via| via.layer_bbox(m2))
                .reduce(|a, b| a.union(b))
                .unwrap()
                .into_rect();

            ctx.draw_rect(m2, port_rect);

            if port == "en_b" {
                let between_transistors = blinv.layer_bbox(nwell).into_rect().bottom();
                let m1_rect = vias[1].layer_bbox(m1).into_rect();
                let m1_rect = m1_rect.with_vspan(m1_rect.vspan().add_point(between_transistors));
                ctx.draw_rect(m1, m1_rect);
                let via_rect = m1_rect.with_vspan(Span::from_center_span_gridded(
                    between_transistors,
                    140,
                    ctx.pdk().layout_grid(),
                ));
                let viap = ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(via_rect, via_rect)
                    .expand(ViaExpansion::LongerDirection)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw_ref(&via)?;
                ctx.add_port(
                    CellPort::builder()
                        .id(port)
                        .add(m2, via.layer_bbox(m2).into_rect())
                        .build(),
                )?;
            } else {
                ctx.add_port(CellPort::builder().id(port).add(m2, port_rect).build())?;
            }
        }

        for port in ["vdd", "vss"] {
            let power_span = Span::from_center_span_gridded(
                blinv.port(port)?.largest_rect(m0)?.center().y,
                POWER_HEIGHT,
                ctx.pdk().layout_grid(),
            );
            let power_stripe = Rect::from_spans(hspan, power_span);
            for inst in [&blinv, &brinv] {
                let pwr = inst.port(port)?.largest_rect(m0)?;
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
            ctx.draw_rect(m2, power_stripe);
            ctx.merge_port(CellPort::with_shape(port, m2, power_stripe));
        }

        ctx.add_port(
            CellPort::builder()
                .id("data_b")
                .add(m0, blinv.port("din")?.largest_rect(m0)?)
                .build(),
        )?;
        ctx.add_port(
            CellPort::builder()
                .id("data")
                .add(m0, brinv.port("din")?.largest_rect(m0)?)
                .build(),
        )?;

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
        let nwell_vspan = blinv.layer_bbox(nwell).into_rect().vspan();
        ctx.draw(blinv)?;
        ctx.draw(brinv)?;
        for port in ["outp", "outn"] {
            let sa_out = sa.port(port)?.largest_rect(m1)?;
            let m1_rect = Rect::from_spans(sa_out.hspan(), ctx.brect().vspan());
            ctx.draw_rect(m1, m1_rect);
        }
        ctx.draw_rect(
            nwell,
            Rect::from_spans(hspan, nwell_vspan.add_point(ctx.brect().top())),
        );
        Ok(())
    }
}

pub struct WriteDriverCent {
    params: WriteDriverParams,
}

impl Component for WriteDriverCent {
    type Params = WriteDriverParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_driver_cent")
    }

    fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
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
            .run_script::<ColumnDesignScript>(&NoParams)?;

        let sa = ctx.instantiate::<WriteDriver>(&self.params)?;
        let hspan = Span::new(0, pc.tap_width);
        let bounds = Rect::from_spans(hspan, sa.brect().vspan());

        ctx.draw_rect(
            nwell,
            Rect::from_spans(hspan, sa.layer_bbox(nwell).into_rect().vspan()),
        );

        let nspan = sa.layer_bbox(nsdm).into_rect().vspan();
        let pspan = sa.layer_bbox(psdm).into_rect().vspan();

        for (span, vdd) in [(pspan, true), (nspan, false)] {
            let r = Rect::from_spans(hspan, span).shrink(200);
            let viap = ViaParams::builder().layers(tap, m0).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
            let sdm_rect = via.layer_bbox(tap).into_rect().expand(130);
            ctx.draw_rect(if vdd { nsdm } else { psdm }, sdm_rect);

            let pspan = sa
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
