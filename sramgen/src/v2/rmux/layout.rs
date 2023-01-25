use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::geom::bbox::{Bbox, BoundBox, LayerBoundBox};
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::{Corner, Dir, Point, Rect, Side, Span};
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

use crate::v2::precharge::Precharge;

use super::{ReadMux, ReadMuxCent, ReadMuxEnd};

const GATE_LINE: i64 = 320;
const GATE_SPACE: i64 = 180;

impl ReadMux {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx
            .inner()
            .run_script::<crate::v2::precharge::layout::PhysicalDesignScript>(&NoParams)?;

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
                    w: self.params.width,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
                MosParams {
                    w: self.params.width,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: mos.id(),
                },
            ],
        };

        let mut mos = ctx.instantiate::<LayoutMos>(&params)?;
        mos.set_orientation(Named::R90);

        mos.place_center(Point::new(pc.width / 2, mos.bbox().height() / 2));
        ctx.draw_ref(&mos)?;

        // Below this, routing follows `in_tracks`.
        let in_top = mos.brect().top() - 600;

        let jog = SimpleJog::builder()
            .dir(Dir::Vert)
            .src_pos(in_top)
            .src([pc.in_tracks.index(1), pc.in_tracks.index(2)])
            .dst([pc.out_tracks.index(0), pc.out_tracks.index(2)])
            .line(pc.v_line)
            .space(pc.v_space)
            .layer(pc.v_metal)
            .build()
            .unwrap();

        // Above this, routing follows `out_tracks`.
        let out_top = jog.dst_pos();
        ctx.draw(jog)?;

        // Below this, routing follows `out_tracks`.
        let out_bot = 600;

        let jog = SimpleJog::builder()
            .dir(Dir::Vert)
            .src_pos(out_bot)
            .src([pc.out_tracks.index(0), pc.out_tracks.index(2)])
            .dst([pc.in_tracks.index(1), pc.in_tracks.index(2)])
            .line(pc.v_line)
            .space(pc.v_space)
            .layer(pc.v_metal)
            .build()
            .unwrap();

        // Above this, routing follows `in_tracks`.
        let in_bot = jog.dst_pos();
        ctx.draw(jog)?;

        let stripe_hspan = Span::new(-pc.width, 2 * pc.width);
        let abs_bot = -(GATE_LINE + GATE_SPACE) * self.params.mux_ratio as i64;

        for i in [0, 2] {
            let top = mos.brect().top();
            let rect = Rect::from_spans(pc.out_tracks.index(i), Span::new(out_top, top));
            ctx.draw_rect(pc.v_metal, rect);

            let rect = Rect::from_spans(pc.out_tracks.index(i), Span::new(abs_bot, out_bot));
            ctx.draw_rect(pc.v_metal, rect);
        }

        let mut tracks = Vec::with_capacity(pc.in_tracks.len());
        for i in 0..pc.in_tracks.len() {
            let rect = Rect::from_spans(pc.in_tracks.index(i), Span::new(in_bot, in_top));
            ctx.draw_rect(pc.v_metal, rect);
            tracks.push(rect);
        }

        for (port, idx) in [("sd_1_1", 1), ("sd_0_0", 2)] {
            let target = mos.port(port)?.largest_rect(pc.m0)?;
            let viap = ViaParams::builder()
                .layers(pc.m0, pc.v_metal)
                .geometry(tracks[idx], target)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
        }

        for (port, idx, x, side) in [
            ("sd_0_1", 0, 0, Side::Left),
            ("sd_1_0", 3, pc.width, Side::Right),
        ] {
            let target = mos.port(port)?.largest_rect(pc.m0)?;
            let viap = ViaParams::builder()
                .layers(pc.m0, pc.v_metal)
                .geometry(
                    Rect::from_xy(x, target.center().y),
                    tracks[idx].double(side),
                )
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_rect(
                pc.m0,
                Rect::from_spans(
                    target.hspan().union(via.brect().hspan()),
                    via.brect().vspan(),
                ),
            );
            ctx.draw_ref(&via)?;

            let stripe = Rect::from_spans(
                stripe_hspan,
                Span::from_center_span_gridded(via.brect().center().y, 600, pc.grid),
            );

            let viap = ViaParams::builder()
                .layers(pc.v_metal, pc.h_metal)
                .geometry(tracks[idx].double(side), stripe)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;

            ctx.draw_rect(pc.h_metal, stripe);
        }

        assert!(self.params.idx < self.params.mux_ratio);

        for i in 0..self.params.mux_ratio {
            let vspan = Span::with_stop_and_length(-(GATE_LINE + GATE_SPACE) * i as i64, GATE_LINE);
            let rect = Rect::from_spans(stripe_hspan, vspan);
            ctx.draw_rect(pc.h_metal, rect);

            if i == self.params.idx {
                let target = mos.port("gate_0")?.largest_rect(pc.m0)?;
                let gate_conn = Rect::from_spans(target.hspan(), target.vspan().union(rect.vspan()));

                let viap = ViaParams::builder()
                    .layers(pc.v_metal, pc.h_metal)
                    .geometry(Rect::from_spans(pc.out_tracks.index(1), rect.vspan()), rect)
                    .build();
                let mut via = ctx.instantiate::<Via>(&viap)?;
                via.place_center(rect.center());
                ctx.draw_ref(&via)?;

                let viap = ViaParams::builder()
                    .layers(pc.m0, pc.v_metal)
                    .geometry(gate_conn, Rect::from_spans(pc.out_tracks.index(1), rect.vspan()))
                    .build();
                let mut via = ctx.instantiate::<Via>(&viap)?;
                via.place_center(rect.center());
                ctx.draw_ref(&via)?;

                ctx.draw_rect(
                    pc.m0,
                    gate_conn
                );
            }
        }

        let power_stripe = Rect::from_spans(stripe_hspan, Span::new(2_200, 3_000));
        ctx.draw_rect(pc.h_metal, power_stripe);

        let bounds = Rect::from_spans(Span::new(0, pc.width), ctx.brect().vspan());
        ctx.flatten();
        ctx.trim(&bounds);

        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;

        ctx.draw_rect(nwell, bounds);
        ctx.draw_rect(psdm, bounds);

        Ok(())
    }
}

impl ReadMuxCent {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}

impl ReadMuxEnd {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}
