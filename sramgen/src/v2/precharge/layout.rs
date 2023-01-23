use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::layout::cell::Port;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::geom::bbox::{Bbox, BoundBox};
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::{Dir, Point, Rect, Span};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::routing::manual::jog::SimpleJog;
use substrate::layout::routing::tracks::{Boundary, CenteredTrackParams, FixedTracks};
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};
use substrate::script::Script;

use super::Precharge;

impl Precharge {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx.inner().run_script::<PhysicalDesignScript>(&NoParams)?;
        let db = ctx.mos_db();
        let mos = db
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())
            .unwrap();
        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 3],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: self.params.equalizer_width as i64,
                    l: self.params.length as i64,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
                MosParams {
                    w: self.params.pull_up_width as i64,
                    l: self.params.length as i64,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
                MosParams {
                    w: self.params.pull_up_width as i64,
                    l: self.params.length as i64,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
            ],
        };

        let gate_stripe = Rect::from_spans(Span::new(0, dsn.width), dsn.gate_stripe);
        ctx.draw_rect(dsn.h_metal, gate_stripe);

        let mut mos = ctx.instantiate::<LayoutMos>(&params)?;
        mos.set_orientation(Named::R90);

        let bbox = Rect::new(Point::zero(), Point::new(dsn.width, mos.bbox().height()));
        mos.align_centers_gridded(bbox.bbox(), dsn.grid);
        mos.align_above(gate_stripe.bbox(), 0);
        ctx.draw_ref(&mos)?;

        let bbox = ctx.bbox().into_rect();

        let cut = mos.port("sd_0_0")?.bbox(dsn.m0).into_rect().top();
        let gate = mos.port("gate_0")?.largest_rect(dsn.m0)?;

        let mut orects = Vec::with_capacity(dsn.out_tracks.len());
        for i in 0..dsn.out_tracks.len() {
            let top = if i == 1 { gate.top() } else { cut };
            let rect = Rect::from_spans(dsn.out_tracks.index(i), Span::new(0, top));
            ctx.draw_rect(dsn.v_metal, rect);
            orects.push(rect);
        }

        let jog = SimpleJog::builder()
            .dir(Dir::Vert)
            .src_pos(cut)
            .src([dsn.out_tracks.index(0), dsn.out_tracks.index(2)])
            .dst([dsn.in_tracks.index(1), dsn.in_tracks.index(2)])
            .line(140)
            .space(140)
            .layer(dsn.v_metal)
            .build()
            .unwrap();

        let mut rects = vec![];
        for i in 0..dsn.in_tracks.len() {
            let rect = Rect::from_spans(
                dsn.in_tracks.index(i),
                Span::new(jog.dst_pos(), bbox.height()),
            );
            rects.push(rect);
            ctx.draw_rect(dsn.v_metal, rect);
        }

        ctx.draw(jog)?;

        let mut params = ViaParams::builder()
            .layers(dsn.m0, dsn.v_metal)
            .geometry(mos.port("sd_1_0")?.largest_rect(dsn.m0).unwrap(), rects[2])
            .build();

        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        let target = mos.port("sd_2_1")?.largest_rect(dsn.m0).unwrap();
        params.set_geometry(target, rects[1]);
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        let vdd_top = mos.port("sd_2_0")?.largest_rect(dsn.m0).unwrap();
        params.set_geometry(
            Bbox::from_point(Point::new(dsn.width, vdd_top.center().y)),
            rects[3],
        );
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(vdd_top.hspan().start(), via.bbox().p1.x),
                via.bbox().into_rect().vspan(),
            ),
        );
        ctx.draw(via)?;

        let vdd_bot = mos.port("sd_1_1")?.largest_rect(dsn.m0).unwrap();
        params.set_geometry(
            Bbox::from_point(Point::new(0, vdd_bot.center().y)),
            rects[0],
        );
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(via.bbox().p0.x, vdd_bot.hspan().stop()),
                via.bbox().into_rect().vspan(),
            ),
        );
        ctx.draw(via)?;

        let bl_bot = mos.port("sd_0_0")?.largest_rect(dsn.m0)?;
        params.set_geometry(
            Bbox::from_point(Point::new(orects[0].center().x, bl_bot.center().y)),
            orects[0],
        );
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        let br_bot = mos.port("sd_0_1")?.largest_rect(dsn.m0)?;
        params.set_geometry(
            Bbox::from_point(Point::new(orects[2].center().x, br_bot.center().y)),
            orects[2],
        );
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        params.set_geometry(gate_stripe, orects[1]);
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        let bbox = ctx.bbox().into_rect();
        let bbox = Rect::from_spans(Span::new(0, dsn.width), bbox.vspan());

        let stripe = Rect::from_spans(bbox.hspan(), dsn.power_stripe);
        ctx.draw_rect(dsn.h_metal, stripe);

        // Effective stripe
        let seff = Rect::from_spans(Span::new(-10_000, 10_000), stripe.vspan());

        let reff = Rect::from_spans(
            Span::with_stop_and_length(rects[0].p1.x, 2 * rects[0].width()),
            rects[0].vspan(),
        );

        let mut params = ViaParams::builder()
            .layers(dsn.v_metal, dsn.h_metal)
            .geometry(reff, seff)
            .build();
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        let reff = Rect::from_spans(
            Span::with_start_and_length(rects[3].p0.x, 2 * rects[3].width()),
            rects[3].vspan(),
        );
        params.set_geometry(reff, seff);
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        params.set_geometry(orects[1], gate_stripe);
        let via = ctx.instantiate::<Via>(&params)?;
        ctx.draw(via)?;

        ctx.draw_rect(dsn.m0, orects[1]);

        ctx.flatten();
        ctx.trim(&bbox);
        Ok(())
    }
}

pub struct PhysicalDesignScript;

pub struct PhysicalDesign {
    /// Location of the horizontal power strap
    power_stripe: Span,
    gate_stripe: Span,
    h_metal: LayerKey,
    width: i64,
    in_tracks: FixedTracks,
    out_tracks: FixedTracks,
    v_metal: LayerKey,
    m0: LayerKey,
    grid: i64,
}

impl Script for PhysicalDesignScript {
    type Params = NoParams;
    type Output = PhysicalDesign;

    fn run(
        _params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let in_tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 140,
            space: 230,
            span: Span::new(0, 1_200),
            num: 4,
            lower_boundary: Boundary::HalfTrack,
            upper_boundary: Boundary::HalfTrack,
            grid: 5,
        });
        let out_tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 140,
            space: 230,
            span: Span::new(0, 1_200),
            num: 3,
            lower_boundary: Boundary::HalfSpace,
            upper_boundary: Boundary::HalfSpace,
            grid: 5,
        });

        let power_stripe = Span::new(3_400, 3_800);
        let gate_stripe = Span::new(0, 360);

        Ok(PhysicalDesign {
            power_stripe,
            gate_stripe,
            h_metal: m2,
            width: 1_200,
            v_metal: m1,
            in_tracks,
            out_tracks,
            grid: 5,
            m0,
        })
    }
}
