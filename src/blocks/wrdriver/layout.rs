use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Dir, Point, Rect, Side, Sign, Span};
use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Port, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;

use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::{OffsetJog, SJog};

use substrate::layout::routing::tracks::{Boundary, FixedTracks};
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

use crate::blocks::delay_line::tristate::TristateInv;
use crate::blocks::gate::{And2, AndParams, PrimitiveGateParams};
use crate::blocks::macros::SenseAmp;

use super::WriteDriver;

use derive_builder::Builder;

const GATE_SPACE: i64 = 210;
const IMPLANT_PAD: i64 = 400;

impl WriteDriver {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx
            .inner()
            .run_script::<crate::blocks::precharge::layout::PhysicalDesignScript>(&NoParams)?;

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
        let logic_params = PrimitiveGateParams {
            pwidth: self.params.pwidth_logic,
            nwidth: self.params.nwidth_logic,
            length: self.params.length,
        };

        // let mut outs = [None, None];
        let stripe_width = 340;
        let stripe_space = 160;
        // let stripe_span = Span::new(-self.params.width, 2 * self.params.width);

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
                .expand(ViaExpansion::LongerDirection)
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
                .into_iter()
                .map(|via| via.layer_bbox(m2))
                .reduce(|a, b| a.union(b))
                .unwrap()
                .into_rect();

            ctx.draw_rect(m2, port_rect);
            ctx.add_port(CellPort::builder().id(port).add(m2, port_rect).build())?;
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
        // let mut cols = Vec::with_capacity(2);
        // for j in 0..2 {
        //     for (i, out) in outs.iter_mut().enumerate() {
        //         let mut inv = ctx.instantiate::<LayoutMos>(&params)?;
        //         inv.place_center_x(j * (inv.brect().width() + 2 * 170));
        //         if i == 0 {
        //             inv.place_center_y(self.params.width / 4);
        //         } else {
        //             inv.orientation_mut().reflect_vert();
        //             inv.place_center_y(3 * self.params.width / 4);
        //         }

        //         for elem in inv.cell().elems() {
        //             if elem.layer.layer() == nwell {
        //                 let elem = elem.transform(inv.transformation());
        //                 let rect = Rect::from_spans(elem.brect().hspan(), stripe_span);
        //                 ctx.draw_rect(nwell, rect);
        //             }
        //         }
        //         col.inv(inv.clone());

        //         let src = inv.port("sd_0_0")?.largest_rect(m0)?;
        //         let dst = inv.port("sd_1_0")?.largest_rect(m0)?;
        //         let short = src.bbox().union(dst.bbox()).into_rect();
        //         ctx.draw_rect(m0, short);
        //         if j == 0 {
        //             *out = Some(short);
        //         }

        //         for (port, name) in [("sd_0_1", "vss"), ("sd_1_1", "vdd")] {
        //             let pwr = inv.port(port)?.largest_rect(m0)?;
        //             let viap = ViaParams::builder()
        //                 .layers(m0, m1)
        //                 .geometry(pwr, pwr)
        //                 .expand(ViaExpansion::LongerDirection)
        //                 .build();
        //             let via = ctx.instantiate::<Via>(&viap)?;
        //             ctx.draw_ref(&via)?;

        //             let power_span =
        //                 Span::from_center_span_gridded(via.brect().center().x, POWER_HEIGHT, GRID);
        //             let power_stripe = Rect::from_spans(power_span, stripe_span);
        //             let viap = ViaParams::builder()
        //                 .layers(m1, m2)
        //                 .geometry(via.layer_bbox(m1), power_stripe)
        //                 .expand(ViaExpansion::LongerDirection)
        //                 .build();
        //             let via = ctx.instantiate::<Via>(&viap)?;
        //             ctx.draw(via)?;
        //             if i == 0 {
        //                 ctx.draw_rect(m2, power_stripe);
        //                 ctx.merge_port(CellPort::with_shape(name, m2, power_stripe));
        //                 if name == "vss" {
        //                     col.vss(power_stripe.hspan());
        //                 } else if name == "vdd" {
        //                     col.vdd(power_stripe.hspan());
        //                 } else {
        //                     unreachable!()
        //                 }
        //             }
        //         }

        //         if j == 1 {
        //             let dst = inv.port("gate_0")?.largest_rect(m0)?;
        //             let jog = SJog::builder()
        //                 .src(out.unwrap())
        //                 .dst(dst)
        //                 .dir(Dir::Horiz)
        //                 .layer(m0)
        //                 .width(170)
        //                 .grid(GRID)
        //                 .build()
        //                 .unwrap();
        //             ctx.draw(jog)?;

        //             let extent = short.right() + 2 * stripe_width + 2 * stripe_space + 40;
        //             let m0_conn = Rect::new(
        //                 short.corner(Corner::LowerLeft),
        //                 Point::new(extent, short.top()),
        //             );
        //             ctx.draw_rect(m0, m0_conn);
        //             let out_span = Span::with_start_and_length(
        //                 short.right() + stripe_space + i as i64 * (stripe_width + stripe_space),
        //                 stripe_width,
        //             );
        //             let stripe = Rect::from_spans(out_span, stripe_span);
        //             ctx.draw_rect(m2, stripe);
        //             let name = if i == 0 {
        //                 arcstr::literal!("outn")
        //             } else {
        //                 arcstr::literal!("outp")
        //             };
        //             ctx.add_port(CellPort::with_shape(name, m2, stripe))
        //                 .unwrap();

        //             let viap = ViaParams::builder()
        //                 .layers(m0, m1)
        //                 .geometry(m0_conn, m0_conn)
        //                 .expand(ViaExpansion::LongerDirection)
        //                 .build();
        //             let via = ctx.instantiate::<Via>(&viap)?;
        //             ctx.draw_ref(&via)?;

        //             let viap = ViaParams::builder()
        //                 .layers(m1, m2)
        //                 .geometry(via.layer_bbox(m1), stripe)
        //                 .expand(ViaExpansion::LongerDirection)
        //                 .build();
        //             let via = ctx.instantiate::<Via>(&viap)?;
        //             ctx.draw_ref(&via)?;
        //         } else {
        //             let input = inv.port("gate_0")?.largest_rect(m0)?;
        //             let extent = input.left() - 2 * stripe_space - 2 * stripe_width;
        //             let m0_conn = Rect::new(Point::new(extent - 40, input.bottom()), input.p1);
        //             let in_span = Span::with_start_and_length(
        //                 extent + i as i64 * (stripe_space + stripe_width),
        //                 stripe_width,
        //             );
        //             ctx.draw_rect(m0, m0_conn);
        //             let stripe = Rect::from_spans(in_span, stripe_span);
        //             ctx.draw_rect(m2, stripe);
        //             let name = if i == 0 {
        //                 arcstr::literal!("inn")
        //             } else {
        //                 arcstr::literal!("inp")
        //             };
        //             ctx.add_port(CellPort::with_shape(name, m2, stripe))
        //                 .unwrap();

        //             let viap = ViaParams::builder()
        //                 .layers(m0, m1)
        //                 .geometry(m0_conn, m0_conn)
        //                 .expand(ViaExpansion::LongerDirection)
        //                 .build();
        //             let via = ctx.instantiate::<Via>(&viap)?;
        //             ctx.draw_ref(&via)?;

        //             let viap = ViaParams::builder()
        //                 .layers(m1, m2)
        //                 .geometry(via.layer_bbox(m1), stripe)
        //                 .expand(ViaExpansion::LongerDirection)
        //                 .build();
        //             let via = ctx.instantiate::<Via>(&viap)?;
        //             ctx.draw_ref(&via)?;
        //         }
        //         ctx.draw(inv)?;
        //     }
        //     cols.push(col.build().unwrap());
        // }

        // ctx.set_metadata(Metadata { cols });

        // let vspan = Span::new(0, self.params.width);
        // let bounds = Rect::from_spans(ctx.brect().hspan(), vspan);
        // ctx.flatten();
        // ctx.trim(&bounds);

        // let outline = layers.get(Selector::Name("outline"))?;
        // let rect = ctx
        //     .brect()
        //     .expand_dims(Dims::new(WELL_PAD, 0), ExpandMode::UpperRight);
        // ctx.draw_rect(outline, rect);

        Ok(())
    }
}

// #[derive(Debug, Builder)]
// struct Metadata {
//     /// m0 and h_metal gate stripes for data, data_b, and wmask
//     gate_stripes: Vec<(Span, Span)>,
//     /// Horizontal power stripe
//     power_stripe: Span,
//     /// Mux control tracks.
//     ctrl_tracks: FixedTracks,
// }
//
// impl Metadata {
//     pub fn builder() -> MetadataBuilder {
//         MetadataBuilder::default()
//     }
// }
//
// fn write_mux_tap_layout(
//     end: bool,
//     params: &WriteMuxCentParams,
//     ctx: &mut LayoutCtx,
// ) -> substrate::error::Result<()> {
//     let pc = ctx
//         .inner()
//         .run_script::<crate::blocks::precharge::layout::PhysicalDesignScript>(&NoParams)?;
//
//     let mux = ctx.instantiate::<WriteMux>(&params.for_wmux())?;
//     let meta = mux.cell().get_metadata::<Metadata>();
//     let stripe_span = Span::new(-pc.tap_width, 2 * pc.tap_width);
//
//     let hspan = Span::new(0, pc.tap_width);
//     let bounds = Rect::from_spans(hspan, mux.brect().vspan().shrink(Sign::Pos, IMPLANT_PAD));
//
//     let tap_span = Span::from_center_span_gridded(pc.tap_width / 2, 170, pc.grid);
//     let tap_space = tap_span.expand_all(170);
//
//     for (i, (bot_span, top_span)) in meta.gate_stripes.iter().copied().enumerate() {
//         for (j, hspan) in [
//             Span::new(0, tap_space.start()),
//             Span::new(tap_space.stop(), pc.tap_width),
//         ]
//         .into_iter()
//         .enumerate()
//         {
//             if end && j == 0 {
//                 continue;
//             }
//
//             let bot = Rect::from_spans(hspan, bot_span);
//             let top = Rect::from_spans(hspan, top_span);
//             ctx.draw_rect(
//                 pc.m0,
//                 bot.expand_side(if j == 0 { Side::Right } else { Side::Left }, -80),
//             );
//             ctx.draw_rect(pc.h_metal, top);
//             ctx.draw_rect(pc.v_metal, Rect::from_spans(hspan.shrink_all(20), top_span));
//             let viap = ViaParams::builder()
//                 .layers(pc.m0, pc.v_metal)
//                 .geometry(bot, top)
//                 .build();
//             let via = ctx.instantiate::<Via>(&viap)?;
//             ctx.draw(via)?;
//
//             let viap = ViaParams::builder()
//                 .layers(pc.v_metal, pc.h_metal)
//                 .geometry(bot, top)
//                 .build();
//             let via = ctx.instantiate::<Via>(&viap)?;
//             ctx.draw(via)?;
//         }
//
//         let short = (i < 2 && !params.cut_data) || (i == 2 && !params.cut_wmask);
//         if short {
//             let rect = Rect::from_spans(stripe_span, top_span);
//             ctx.draw_rect(pc.h_metal, rect);
//         }
//     }
//
//     let layers = ctx.layers();
//     let tap = layers.get(Selector::Name("tap"))?;
//     let outline = layers.get(Selector::Name("outline"))?;
//
//     let tap_area = Rect::from_spans(tap_span, bounds.vspan().shrink_all(300));
//     let viap = ViaParams::builder()
//         .layers(tap, pc.m0)
//         .geometry(tap_area, tap_area)
//         .expand(ViaExpansion::LongerDirection)
//         .build();
//     let via = ctx.instantiate::<Via>(&viap)?;
//     ctx.draw_ref(&via)?;
//
//     let viap = ViaParams::builder()
//         .layers(pc.m0, pc.v_metal)
//         .geometry(via.layer_bbox(pc.m0), tap_area)
//         .expand(ViaExpansion::LongerDirection)
//         .build();
//     let via = ctx.instantiate::<Via>(&viap)?;
//     ctx.draw_ref(&via)?;
//
//     let power_stripe = Rect::from_spans(stripe_span, meta.power_stripe);
//     ctx.draw_rect(pc.h_metal, power_stripe);
//     ctx.add_port(CellPort::with_shape("vss", pc.h_metal, power_stripe))
//         .unwrap();
//
//     let viap = ViaParams::builder()
//         .layers(pc.v_metal, pc.h_metal)
//         .geometry(via.layer_bbox(pc.v_metal), power_stripe)
//         .expand(ViaExpansion::LongerDirection)
//         .build();
//     let via = ctx.instantiate::<Via>(&viap)?;
//     ctx.draw_ref(&via)?;
//
//     for (i, track) in meta.ctrl_tracks.iter().enumerate() {
//         let rect = Rect::from_spans(hspan, track);
//         ctx.draw_rect(pc.h_metal, rect);
//         ctx.add_port(CellPort::with_shape(PortId::new("we", i), pc.h_metal, rect))?;
//     }
//
//     let bounds = Rect::from_spans(hspan, mux.brect().vspan());
//     let psdm = layers.get(Selector::Name("psdm"))?;
//     ctx.draw_rect(outline, bounds);
//     ctx.draw_rect(psdm, bounds);
//     ctx.flatten();
//     ctx.trim(&bounds);
//
//     Ok(())
// }
//
// impl WriteMuxCent {
//     pub(crate) fn layout(
//         &self,
//         ctx: &mut substrate::layout::context::LayoutCtx,
//     ) -> substrate::error::Result<()> {
//         write_mux_tap_layout(false, &self.params, ctx)?;
//         Ok(())
//     }
// }
//
// impl WriteMuxEnd {
//     pub(crate) fn layout(
//         &self,
//         ctx: &mut substrate::layout::context::LayoutCtx,
//     ) -> substrate::error::Result<()> {
//         write_mux_tap_layout(true, &self.params.for_wmux_cent(), ctx)?;
//         Ok(())
//     }
// }
