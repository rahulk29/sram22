use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::geom::bbox::{Bbox, BoundBox, LayerBoundBox};
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::{Dir, Point, Rect, Side, Span};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::SimpleJog;
use substrate::layout::routing::tracks::{Boundary, CenteredTrackParams, FixedTracks};
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};
use substrate::script::Script;

use super::{Precharge, PrechargeCent, PrechargeEnd};

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
                    w: self.params.equalizer_width,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
                MosParams {
                    w: self.params.pull_up_width,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
                MosParams {
                    w: self.params.pull_up_width,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
            ],
        };

        let stripe_span = Span::new(-dsn.width, 2 * dsn.width);
        let gate_stripe = Rect::from_spans(stripe_span, dsn.gate_stripe);
        ctx.draw_rect(dsn.h_metal, gate_stripe);

        let mut mos = ctx.instantiate::<LayoutMos>(&params)?;
        mos.set_orientation(Named::R90);

        mos.place_center(Point::new(dsn.width / 2, 0));
        mos.align_above(gate_stripe, 0);
        ctx.draw_ref(&mos)?;

        let bbox = ctx.bbox().into_rect();

        let cut = dsn.cut;
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
            .line(dsn.v_line)
            .space(dsn.v_space)
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

        let mut via0 = ViaParams::builder()
            .layers(dsn.m0, dsn.v_metal)
            .geometry(mos.port("sd_1_0")?.largest_rect(dsn.m0)?, rects[2])
            .build();

        let via = ctx.instantiate::<Via>(&via0)?;
        ctx.draw(via)?;

        let target = mos.port("sd_2_1")?.largest_rect(dsn.m0)?;
        via0.set_geometry(target, rects[1]);
        let via = ctx.instantiate::<Via>(&via0)?;
        ctx.draw(via)?;

        let mut m1_vias = Vec::with_capacity(2);
        for (port, rect, x) in [("sd_2_0", rects[3], dsn.width), ("sd_1_1", rects[0], 0)] {
            let port = mos.port(port)?.largest_rect(dsn.m0)?;
            via0.set_geometry(Bbox::from_point(Point::new(x, port.center().y)), rect);
            let via = ctx.instantiate::<Via>(&via0)?;
            ctx.draw_rect(
                dsn.m0,
                Rect::from_spans(port.hspan().union(via.brect().hspan()), via.brect().vspan()),
            );
            ctx.draw(via)?;

            m1_vias.push(via0.clone());
        }

        for (port, rect) in [("sd_0_0", orects[0]), ("sd_0_1", orects[2])] {
            let port = mos.port(port)?.largest_rect(dsn.m0)?;
            via0.set_geometry(
                Bbox::from_point(Point::new(rect.center().x, port.center().y)),
                rect,
            );
            let via = ctx.instantiate::<Via>(&via0)?;
            ctx.draw(via)?;
        }

        via0.set_geometry(gate_stripe, orects[1]);
        let via = ctx.instantiate::<Via>(&via0)?;
        ctx.draw(via)?;

        let stripe = Rect::from_spans(stripe_span, dsn.power_stripe);
        ctx.draw_rect(dsn.h_metal, stripe);
        ctx.add_port(CellPort::with_shape("vdd", dsn.h_metal, stripe));

        let mut via1 = ViaParams::builder()
            .layers(dsn.v_metal, dsn.h_metal)
            .expand(ViaExpansion::LongerDirection)
            .geometry(rects[0].double(Side::Left), stripe)
            .build();
        let via = ctx.instantiate::<Via>(&via1)?;
        ctx.draw(via)?;

        let metadata = Metadata {
            m1_via_top: m1_vias[0].clone(),
            m1_via_bot: m1_vias[1].clone(),
            m2_via: via1.clone(),
        };
        ctx.set_metadata(metadata);

        via1.set_geometry(rects[3].double(Side::Right), stripe);
        let via = ctx.instantiate::<Via>(&via1)?;
        ctx.draw(via)?;

        via1.set_geometry(orects[1], gate_stripe);
        let via = ctx.instantiate::<Via>(&via1)?;
        ctx.draw(via)?;

        ctx.draw_rect(dsn.m0, orects[1]);

        ctx.flatten();

        let bounds = ctx.brect().with_hspan(Span::new(0, dsn.width));

        let layers = ctx.layers();
        let psdm = layers.get(Selector::Name("psdm"))?;
        ctx.draw_rect(psdm, bounds);
        ctx.trim(&bounds);
        Ok(())
    }
}

struct Metadata {
    m1_via_bot: ViaParams,
    m1_via_top: ViaParams,
    m2_via: ViaParams,
}

impl PrechargeCent {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx.instantiate::<Precharge>(&self.params)?;
        let dsn = ctx.inner().run_script::<PhysicalDesignScript>(&NoParams)?;
        let meta = pc.cell().get_metadata::<Metadata>();
        let layers = ctx.layers();

        let tap = layers.get(Selector::Name("tap"))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let m0 = layers.get(Selector::Metal(0))?;

        let brect = Rect::new(
            Point::new(0, 0),
            Point::new(dsn.tap_width, pc.brect().top()),
        );

        ctx.draw_rect(nwell, brect);
        ctx.draw_rect(nsdm, brect);

        let tap_rect = brect.shrink(300);

        let viap = ViaParams::builder()
            .layers(tap, m0)
            .geometry(tap_rect, tap_rect)
            .build();
        let tap = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_ref(&tap)?;

        let y = dsn.cut + 2 * dsn.v_line + dsn.v_space;
        let half_tr = Rect::from_spans(Span::new(0, dsn.v_line / 2), Span::new(y, brect.top()));
        ctx.draw_rect(dsn.v_metal, half_tr);

        let mut via = ctx.instantiate::<Via>(&meta.m1_via_top)?;
        via.place_center(Point::new(0, via.brect().center().y));
        ctx.draw(via.clone())?;
        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(0, tap.layer_bbox(dsn.m0).p0.x),
                via.brect().vspan(),
            ),
        );

        via.place_center(Point::new(dsn.tap_width, via.brect().center().y));
        ctx.draw(via.clone())?;

        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(tap.layer_bbox(dsn.m0).p1.x, dsn.tap_width),
                via.brect().vspan(),
            ),
        );

        let half_tr = Rect::from_spans(
            Span::with_stop_and_length(dsn.tap_width, dsn.v_line / 2),
            Span::new(y, brect.top()),
        );
        ctx.draw_rect(dsn.v_metal, half_tr);

        let stripe_span = Span::new(-dsn.tap_width, 2 * dsn.tap_width);
        ctx.draw_rect(dsn.h_metal, Rect::from_spans(stripe_span, dsn.gate_stripe));

        let power_stripe = Rect::from_spans(stripe_span, dsn.power_stripe);
        ctx.draw_rect(dsn.h_metal, power_stripe);
        ctx.add_port(CellPort::with_shape("vdd", dsn.h_metal, power_stripe));

        let viap = ViaParams::builder()
            .layers(dsn.m0, dsn.v_metal)
            .geometry(tap.layer_bbox(dsn.m0), tap.layer_bbox(dsn.m0))
            .build();
        let via = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_ref(&via)?;

        let viap = ViaParams::builder()
            .layers(dsn.v_metal, dsn.h_metal)
            .geometry(via.layer_bbox(dsn.v_metal), power_stripe)
            .build();
        let via = ctx.instantiate::<Via>(&viap)?;
        ctx.draw(via)?;

        let mut via = ctx.instantiate::<Via>(&meta.m2_via)?;
        ctx.draw_ref(&via)?;

        via.place_center(Point::new(dsn.tap_width, via.brect().center().y));
        ctx.draw(via)?;

        ctx.flatten();
        ctx.trim(&brect);

        Ok(())
    }
}

impl PrechargeEnd {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx.instantiate::<Precharge>(&self.params)?;
        let dsn = ctx.inner().run_script::<PhysicalDesignScript>(&NoParams)?;
        let meta = pc.cell().get_metadata::<Metadata>();
        let layers = ctx.layers();

        let tap = layers.get(Selector::Name("tap"))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let m0 = layers.get(Selector::Metal(0))?;

        let brect = Rect::new(
            Point::new(0, 0),
            Point::new(dsn.tap_width, pc.brect().top()),
        );

        ctx.draw_rect(nwell, brect);
        ctx.draw_rect(nsdm, brect);

        let tap_rect = brect.shrink(300);

        let viap = ViaParams::builder()
            .layers(tap, m0)
            .geometry(tap_rect, tap_rect)
            .build();
        let tap = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_ref(&tap)?;

        let y = dsn.cut + 2 * dsn.v_line + dsn.v_space;

        let mut via = ctx.instantiate::<Via>(&meta.m1_via_bot)?;
        via.place_center(Point::new(dsn.tap_width, via.brect().center().y));
        ctx.draw(via.clone())?;
        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(tap.layer_bbox(dsn.m0).p1.x, dsn.tap_width),
                via.brect().vspan(),
            ),
        );

        let half_tr = Rect::from_spans(
            Span::with_stop_and_length(dsn.tap_width, dsn.v_line / 2),
            Span::new(y, brect.top()),
        );
        ctx.draw_rect(dsn.v_metal, half_tr);

        let stripe_span = Span::new(-dsn.tap_width, 2 * dsn.tap_width);
        ctx.draw_rect(dsn.h_metal, Rect::from_spans(stripe_span, dsn.gate_stripe));
        let power_stripe = Rect::from_spans(stripe_span, dsn.power_stripe);
        ctx.draw_rect(dsn.h_metal, power_stripe);

        let viap = ViaParams::builder()
            .layers(dsn.m0, dsn.v_metal)
            .geometry(tap.layer_bbox(dsn.m0), tap.layer_bbox(dsn.m0))
            .build();
        let via = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_ref(&via)?;

        let viap = ViaParams::builder()
            .layers(dsn.v_metal, dsn.h_metal)
            .geometry(via.layer_bbox(dsn.v_metal), power_stripe)
            .build();
        let via = ctx.instantiate::<Via>(&viap)?;
        ctx.draw(via)?;

        let mut via = ctx.instantiate::<Via>(&meta.m2_via)?;
        via.place_center(Point::new(dsn.tap_width, via.brect().center().y));
        ctx.draw(via)?;

        ctx.flatten();
        ctx.trim(&brect);

        Ok(())
    }
}

pub struct PhysicalDesignScript;

pub struct PhysicalDesign {
    /// Location of the horizontal power strap
    pub(crate) power_stripe: Span,
    pub(crate) gate_stripe: Span,
    pub(crate) h_metal: LayerKey,
    pub(crate) cut: i64,
    pub(crate) width: i64,
    pub(crate) in_tracks: FixedTracks,
    pub(crate) out_tracks: FixedTracks,
    pub(crate) v_metal: LayerKey,
    pub(crate) v_line: i64,
    pub(crate) v_space: i64,
    pub(crate) m0: LayerKey,
    pub(crate) grid: i64,
    pub(crate) tap_width: i64,
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

        let power_stripe = Span::new(3_400, 4_200);
        let gate_stripe = Span::new(0, 360);

        Ok(PhysicalDesign {
            power_stripe,
            gate_stripe,
            h_metal: m2,
            cut: 1_920,
            width: 1_200,
            v_metal: m1,
            v_line: 140,
            v_space: 140,
            in_tracks,
            out_tracks,
            grid: 5,
            tap_width: 1_300,
            m0,
        })
    }
}
