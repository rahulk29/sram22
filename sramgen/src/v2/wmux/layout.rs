use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::layout::cell::Port;
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::geom::bbox::BoundBox;
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

use super::{WriteMux, WriteMuxCent, WriteMuxEnd, WriteMuxParams};

use derive_builder::Builder;

const GATE_LINE: i64 = 320;
const GATE_SPACE: i64 = 180;

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
                w: self.params.mux_width,
                l: self.params.length,
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
        wmask.align_beneath(mux2.bbox(), 300);
        ctx.draw_ref(&wmask)?;

        let mut npd = ctx.instantiate::<LayoutMos>(&params)?;
        npd.set_orientation(Named::R90);
        npd.place_center_x(cx);
        npd.align_beneath(wmask.bbox(), 300);
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
            .map(|track| {
                let r =
                    Rect::from_spans(track, Span::new(mux2.brect().bottom(), ctx.brect().top()));
                ctx.draw_rect(pc.v_metal, r);
                r
            })
            .collect::<Vec<_>>();

        let target = npd.port("sd_0_1")?.largest_rect(pc.m0)?;
        let jog = SJog::builder()
            .src(tracks[1])
            .dst(target)
            .layer(pc.v_metal)
            .dir(Dir::Vert)
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
        let power_span = Span::from_center_span_gridded(target.center().y, 600, pc.grid);
        let stripe_span = Span::new(-pc.width, 2 * pc.width);

        let power_stripe = Rect::from_spans(stripe_span, power_span);
        ctx.draw_rect(pc.h_metal, power_stripe);

        let target = npd.port("sd_0_0")?.largest_rect(pc.m0)?;
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

        for mux in [&mux1, &mux2] {
            let target = mux.port("gate_0")?.largest_rect(pc.m0)?;
            let rect = Rect::from_spans(stripe_span, target.vspan());
            ctx.draw_rect(pc.m0, rect);

            let span = Span::from_center_span_gridded(target.center().y, 340, pc.grid);
            let rect = Rect::from_spans(stripe_span, span);
            ctx.draw_rect(pc.h_metal, rect);
        }

        let tracks = FixedTracks {
            line: 340,
            space: 160,
            boundary_space: 160,
            interior_tracks: self.params.mux_ratio.checked_sub(2).unwrap(),
            start: npd.brect().bottom(),
            lower_boundary: Boundary::Track,
            upper_boundary: Boundary::Track,
            sign: Sign::Neg,
        };

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
    data_stripe: Span,
    data_b_stripe: Span,
    pd_stripe: Span,
    power_stripe: Span,
}

impl Metadata {
    pub fn builder() -> MetadataBuilder {
        MetadataBuilder::default()
    }
}

impl WriteMuxCent {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}

fn write_mux_tap_layout(
    _width: i64,
    _end: bool,
    _params: &WriteMuxParams,
    _ctx: &mut LayoutCtx,
) -> substrate::error::Result<()> {
    Ok(())
}

impl WriteMuxEnd {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}
