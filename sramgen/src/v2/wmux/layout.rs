use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::geom::bbox::{BoundBox, LayerBoundBox};
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::{Dir, Point, Rect, Sign, Span};
use substrate::layout::layers::selector::Selector;

use substrate::layout::placement::align::AlignRect;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::{ElbowJog, SJog};

use substrate::layout::routing::tracks::{Boundary, FixedTracks};
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

use super::{WriteMux, WriteMuxCent, WriteMuxCentParams, WriteMuxEnd};

use derive_builder::Builder;

const GATE_SPACE: i64 = 160;

impl WriteMux {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx
            .inner()
            .run_script::<crate::v2::precharge::layout::PhysicalDesignScript>(&NoParams)?;

        let db = ctx.mos_db();
        let mos = db
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())
            .unwrap();

        let cx = pc.width / 2;
        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 2],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![MosParams {
                w: self.params.sizing.mux_width,
                l: self.params.sizing.length,
                m: 1,
                nf: 1,
                id: mos.id(),
            }],
        };

        let _meta = Metadata::builder();

        let mut mux1 = ctx.instantiate::<LayoutMos>(&params)?;
        mux1.set_orientation(Named::R90);
        mux1.place_center_x(cx);
        ctx.draw_ref(&mux1)?;
        let mut mux2 = ctx.instantiate::<LayoutMos>(&params)?;
        mux2.set_orientation(Named::R90);
        mux2.align_beneath(mux1.bbox(), 400);
        mux2.place_center_x(cx);
        ctx.draw_ref(&mux2)?;

        let viap = ViaParams::builder()
            .layers(pc.m0, pc.v_metal)
            .geometry(
                mux1.port("gate_0")?.largest_rect(pc.m0)?,
                Rect::from_spans(pc.out_tracks.index(1), Span::new(-10_000, 10_000)),
            )
            .build();
        let mut via = ctx.instantiate::<Via>(&viap)?;

        let src = mux1.port("sd_0_0")?.largest_rect(pc.m0)?;
        let elbow = ElbowJog::builder()
            .dir(Dir::Vert)
            .sign(Sign::Pos)
            .src(src)
            .dst(pc.out_tracks.index(1).start() - 40)
            .layer(pc.m0)
            .build()
            .unwrap();
        ctx.draw_ref(&elbow)?;

        via.place_center(Point::new(
            pc.out_tracks.index(1).center(),
            elbow.r2().center().y,
        ));
        ctx.draw_ref(&via)?;

        let src = mux2.port("sd_0_1")?.largest_rect(pc.m0)?;
        let elbow = ElbowJog::builder()
            .dir(Dir::Vert)
            .sign(Sign::Pos)
            .src(src)
            .dst(pc.out_tracks.index(1).stop() + 40)
            .layer(pc.m0)
            .build()
            .unwrap();
        ctx.draw_ref(&elbow)?;

        via.place_center(Point::new(
            pc.out_tracks.index(1).center(),
            elbow.r2().center().y,
        ));
        ctx.draw_ref(&via)?;

        let mut wmask = ctx.instantiate::<LayoutMos>(&params)?;
        wmask.set_orientation(Named::R90);
        wmask.place_center_x(cx);
        wmask.align_beneath(mux2.bbox(), GATE_SPACE);
        ctx.draw_ref(&wmask)?;

        let mut npd = ctx.instantiate::<LayoutMos>(&params)?;
        npd.set_orientation(Named::R90);
        npd.place_center_x(cx);
        npd.align_beneath(wmask.bbox(), GATE_SPACE);
        ctx.draw_ref(&npd)?;

        for (inst, port, idx) in [(&mux1, "sd_0_1", 0), (&mux2, "sd_0_0", 2)] {
            let target = inst.port(port)?.largest_rect(pc.m0)?;
            via.place_center(Point::new(
                pc.out_tracks.index(idx).center(),
                target.center().y,
            ));
            ctx.draw_ref(&via)?;
            let rect = Rect::from_spans(
                via.brect().hspan().union(target.hspan()),
                via.brect().vspan(),
            );
            ctx.draw_rect(pc.m0, rect);
        }

        let tracks = pc
            .out_tracks
            .iter()
            .map(|track| -> Result<Rect, substrate::error::SubstrateError> {
                let r = Rect::from_spans(
                    track,
                    Span::new(
                        mux2.port("gate_0")?.largest_rect(pc.m0)?.top(),
                        ctx.brect().top(),
                    ),
                );
                ctx.draw_rect(pc.v_metal, r);
                Ok(r)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let target = wmask.port("sd_0_0")?.largest_rect(pc.m0)?;
        let jog = SJog::builder()
            .src(tracks[1])
            .dst(target)
            .layer(pc.v_metal)
            .dir(Dir::Vert)
            .grid(pc.grid)
            .build()
            .unwrap();
        ctx.draw_ref(&jog)?;

        let viap = ViaParams::builder()
            .layers(pc.m0, pc.v_metal)
            .geometry(target, jog.r3())
            .expand(ViaExpansion::LongerDirection)
            .build();
        let via_arr = ctx.instantiate::<Via>(&viap)?;
        ctx.draw(via_arr)?;

        let target = npd.port("sd_0_0")?.largest_rect(pc.m0)?;
        let power_span = Span::from_center_span_gridded(target.center().y, 800, pc.grid);
        let stripe_span = Span::new(-pc.width, 2 * pc.width);

        let power_stripe = Rect::from_spans(stripe_span, power_span);
        ctx.draw_rect(pc.h_metal, power_stripe);
        ctx.add_port(CellPort::with_shape("vss", pc.h_metal, power_stripe));
        let viap = ViaParams::builder()
            .layers(pc.m0, pc.v_metal)
            .geometry(target, target)
            .expand(ViaExpansion::LongerDirection)
            .build();
        let v = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_ref(&v)?;
        let viap = ViaParams::builder()
            .layers(pc.v_metal, pc.h_metal)
            .geometry(v.brect(), power_stripe)
            .expand(ViaExpansion::LongerDirection)
            .build();
        let v = ctx.instantiate::<Via>(&viap)?;
        ctx.draw(v)?;

        let mut gate_stripes = Vec::with_capacity(3);
        for (inst, port) in [(&mux1, "data"), (&mux2, "data_b"), (&wmask, "wmask")] {
            let target = inst.port("gate_0")?.largest_rect(pc.m0)?;
            let rect = Rect::from_spans(stripe_span, target.vspan());
            ctx.draw_rect(pc.m0, rect);

            let span = Span::from_center_span_gridded(target.center().y, 340, pc.grid);
            let rect = Rect::from_spans(stripe_span, span);
            ctx.draw_rect(pc.h_metal, rect);
            gate_stripes.push((target.vspan(), span));

            ctx.add_port(CellPort::with_shape(port, pc.h_metal, rect));

            if std::ptr::eq(inst, &wmask) {
                via.place_center(Point::new(cx + 220, target.center().y));
                ctx.draw_ref(&via)?;

                let viap = ViaParams::builder()
                    .layers(pc.v_metal, pc.h_metal)
                    .geometry(via.brect(), rect)
                    .build();
                let v = ctx.instantiate::<Via>(&viap)?;
                ctx.draw(v)?;
            }
        }

        let src = wmask.port("sd_0_1")?.largest_rect(pc.m0)?;
        let dst = npd.port("sd_0_1")?.largest_rect(pc.m0)?;
        let rect = src.bbox().union(dst.bbox()).into_rect();
        ctx.draw_rect(pc.v_metal, rect);

        for inst in [&wmask, &npd] {
            let bot = inst.port("sd_0_1")?.largest_rect(pc.m0)?;
            let viap = ViaParams::builder()
                .geometry(bot, rect)
                .layers(pc.m0, pc.v_metal)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
        }

        let tracks = FixedTracks {
            line: 340,
            space: 160,
            boundary_space: 160,
            interior_tracks: self.params.sizing.mux_ratio.checked_sub(2).unwrap(),
            start: npd.brect().bottom(),
            lower_boundary: Boundary::Track,
            upper_boundary: Boundary::Track,
            sign: Sign::Neg,
        };

        ctx.set_metadata(Metadata {
            gate_stripes,
            power_stripe: power_stripe.vspan(),
            ctrl_tracks: tracks.clone(),
        });

        for (i, track) in tracks.iter().enumerate() {
            let rect = Rect::from_spans(stripe_span, track);
            ctx.draw_rect(pc.h_metal, rect);

            if i == self.params.idx {
                let target = npd.port("gate_0")?.largest_rect(pc.m0)?;
                let gate_conn =
                    Rect::from_spans(target.hspan(), target.vspan().union(rect.vspan()));

                let viap = ViaParams::builder()
                    .layers(pc.v_metal, pc.h_metal)
                    .geometry(Rect::from_spans(pc.out_tracks.index(1), rect.vspan()), rect)
                    .build();
                let mut via1 = ctx.instantiate::<Via>(&viap)?;
                via1.place_center(rect.center());
                ctx.draw_ref(&via1)?;

                via.place_center(rect.center());
                ctx.draw_ref(&via)?;

                ctx.draw_rect(pc.m0, gate_conn);
            }
        }

        let layers = ctx.layers();
        let nsdm = layers.get(Selector::Name("nsdm"))?;

        let bounds = Rect::from_spans(Span::new(0, pc.width), ctx.brect().vspan());
        ctx.draw_rect(nsdm, bounds);
        ctx.flatten();
        ctx.trim(&bounds);

        Ok(())
    }
}

#[derive(Debug, Builder)]
struct Metadata {
    /// m0 and h_metal gate stripes for data, data_b, and wmask
    gate_stripes: Vec<(Span, Span)>,
    /// Horizontal power stripe
    power_stripe: Span,
    /// Mux control tracks.
    ctrl_tracks: FixedTracks,
}

impl Metadata {
    pub fn builder() -> MetadataBuilder {
        MetadataBuilder::default()
    }
}

fn write_mux_tap_layout(
    end: bool,
    params: &WriteMuxCentParams,
    ctx: &mut LayoutCtx,
) -> substrate::error::Result<()> {
    let pc = ctx
        .inner()
        .run_script::<crate::v2::precharge::layout::PhysicalDesignScript>(&NoParams)?;

    let mux = ctx.instantiate::<WriteMux>(&params.for_wmux())?;
    let meta = mux.cell().get_metadata::<Metadata>();
    let stripe_span = Span::new(-pc.tap_width, 2 * pc.tap_width);

    let hspan = Span::new(0, pc.tap_width);
    let bounds = Rect::from_spans(hspan, mux.brect().vspan());

    let tap_span = Span::from_center_span_gridded(pc.tap_width / 2, 170, pc.grid);
    let tap_space = tap_span.expand_all(170);

    for (i, (bot_span, top_span)) in meta.gate_stripes.iter().copied().enumerate() {
        for (j, hspan) in [
            Span::new(0, tap_space.start()),
            Span::new(tap_space.stop(), pc.tap_width),
        ]
        .into_iter()
        .enumerate()
        {
            if end && j == 0 {
                continue;
            }

            let bot = Rect::from_spans(hspan, bot_span);
            let top = Rect::from_spans(hspan, top_span);
            ctx.draw_rect(pc.m0, bot);
            ctx.draw_rect(pc.h_metal, top);
            ctx.draw_rect(pc.v_metal, Rect::from_spans(hspan.shrink_all(20), top_span));
            let viap = ViaParams::builder()
                .layers(pc.m0, pc.v_metal)
                .geometry(bot, top)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            let viap = ViaParams::builder()
                .layers(pc.v_metal, pc.h_metal)
                .geometry(bot, top)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
        }

        let short = (i < 2 && !params.cut_data) || (i == 2 && !params.cut_wmask);
        if short {
            let rect = Rect::from_spans(stripe_span, top_span);
            ctx.draw_rect(pc.h_metal, rect);
        }
    }

    let layers = ctx.layers();
    let tap = layers.get(Selector::Name("tap"))?;

    let tap_area = Rect::from_spans(tap_span, bounds.vspan().shrink_all(300));
    let viap = ViaParams::builder()
        .layers(tap, pc.m0)
        .geometry(tap_area, tap_area)
        .expand(ViaExpansion::LongerDirection)
        .build();
    let via = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&via)?;

    let viap = ViaParams::builder()
        .layers(pc.m0, pc.v_metal)
        .geometry(via.layer_bbox(pc.m0), tap_area)
        .expand(ViaExpansion::LongerDirection)
        .build();
    let via = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&via)?;

    let power_stripe = Rect::from_spans(stripe_span, meta.power_stripe);
    ctx.draw_rect(pc.h_metal, power_stripe);

    let viap = ViaParams::builder()
        .layers(pc.v_metal, pc.h_metal)
        .geometry(via.layer_bbox(pc.v_metal), power_stripe)
        .expand(ViaExpansion::LongerDirection)
        .build();
    let via = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&via)?;

    for track in meta.ctrl_tracks.iter() {
        let rect = Rect::from_spans(hspan, track);
        ctx.draw_rect(pc.h_metal, rect);
    }

    let psdm = layers.get(Selector::Name("psdm"))?;
    ctx.draw_rect(psdm, bounds);

    ctx.flatten();
    ctx.trim(&bounds);

    Ok(())
}

impl WriteMuxCent {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        write_mux_tap_layout(false, &self.params, ctx)?;
        Ok(())
    }
}

impl WriteMuxEnd {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        write_mux_tap_layout(true, &self.params.for_wmux_cent(), ctx)?;
        Ok(())
    }
}
