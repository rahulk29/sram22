use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Dir, Point, Rect, Side, Span};
use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Port, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;

use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::SimpleJog;

use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

use super::{TGateMux, TGateMuxCent, TGateMuxEnd, TGateMuxGroup, TGateMuxParams};

use crate::blocks::columns::ColumnDesignScript;
use crate::blocks::sram::layout::draw_via;
use derive_builder::Builder;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::ArrayTiler;

const GATE_LINE: i64 = 320;
const GATE_SPACE: i64 = 180;

impl TGateMux {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let db = ctx.mos_db();
        let pmos = db
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())
            .unwrap();
        let nmos = db
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())
            .unwrap();
        let pmos_params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 3],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: self.params.pwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: pmos.id(),
                },
                MosParams {
                    w: self.params.pwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: pmos.id(),
                },
            ],
        };
        let nmos_params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 3],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: self.params.nwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: nmos.id(),
                },
                MosParams {
                    w: self.params.nwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: nmos.id(),
                },
            ],
        };

        let mut pmos = ctx.instantiate::<LayoutMos>(&pmos_params)?;
        pmos.set_orientation(Named::R90);
        let mut nmos = ctx.instantiate::<LayoutMos>(&nmos_params)?;
        nmos.set_orientation(Named::R270);

        pmos.place_center(Point::new(pc.width / 2, pmos.bbox().height() / 2));
        nmos.align_centers_horizontally(&pmos);
        nmos.align_above(&pmos, 400);

        ctx.draw_ref(&pmos)?;
        ctx.draw_ref(&nmos)?;

        // Below this, routing follows `in_tracks`.
        let in_top = nmos.brect().top() - 600;

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

        // The lower extent of BL/BR routing.
        let out_bot = 600;

        let stripe_hspan = Span::new(-pc.width, 2 * pc.width);
        let line = self.params.sel_width;
        let space = std::cmp::max(line / 4, 140);
        let abs_bot = -(line + space) * self.params.mux_ratio as i64;
        let abs_top = nmos.brect().top()
            + (line + space) * self.params.mux_ratio as i64
            + std::cmp::max(170 - space, 0);

        for i in [0, 2] {
            let rect = Rect::from_spans(pc.out_tracks.index(i), Span::new(out_top, abs_top));
            ctx.draw_rect(pc.v_metal, rect);
        }

        let mut tracks = Vec::with_capacity(pc.in_tracks.len());
        for i in 0..pc.in_tracks.len() {
            let bot = if i == 0 || i == 3 { abs_bot } else { out_bot };
            let mut hspan = pc.in_tracks.index(i);
            if i == 0 {
                hspan = hspan.expand(true, 70);
            } else if i == 3 {
                hspan = hspan.expand(false, 70);
            }
            let rect = Rect::from_spans(hspan, Span::new(bot, in_top));
            tracks.push(rect);
        }

        ctx.add_port(CellPort::with_shape("bl", pc.v_metal, tracks[1]))?;
        ctx.add_port(CellPort::with_shape("br", pc.v_metal, tracks[2]))?;

        let mut metadata = Metadata::builder();
        metadata.sel_tracks_ystart(nmos.brect().top());
        metadata.bl_in_top(in_top);

        for (port, idx) in [("sd_1_1", 2), ("sd_0_0", 1)] {
            let target = nmos.port(port)?.largest_rect(pc.m0)?;
            let viap = ViaParams::builder()
                .layers(pc.m0, pc.v_metal)
                .geometry(tracks[idx], target)
                .expand(ViaExpansion::LongerDirection)
                .top_extension(Dir::Vert)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
        }
        for (port, idx) in [("sd_1_1", 1), ("sd_0_0", 2)] {
            let target = pmos.port(port)?.largest_rect(pc.m0)?;
            let viap = ViaParams::builder()
                .layers(pc.m0, pc.v_metal)
                .geometry(tracks[idx], target)
                .expand(ViaExpansion::LongerDirection)
                .top_extension(Dir::Vert)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
        }

        let mut split_via0 = Vec::new();
        let mut split_via1 = Vec::new();
        let mut blbr_stripes = Vec::new();

        for (inst, port, idx, x, side, name) in [
            (&pmos, "sd_1_0", 3, pc.width, Side::Right, Some("bl_out")),
            (&pmos, "sd_0_1", 0, 0, Side::Left, Some("br_out")),
            (&nmos, "sd_0_1", 3, pc.width, Side::Right, None),
            (&nmos, "sd_1_0", 0, 0, Side::Left, None),
        ] {
            let target = inst.port(port)?.largest_rect(pc.m0)?;
            let viap = ViaParams::builder()
                .layers(pc.m0, pc.v_metal)
                .geometry(
                    Rect::from_spans(Span::from_point(x), target.vspan()),
                    tracks[idx].double(side).expand_dir(Dir::Vert, -80),
                )
                .expand(ViaExpansion::LongerDirection)
                .top_extension(Dir::Vert)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            if side == Side::Left {
                split_via0.push(viap);
            }
            ctx.draw_rect(
                pc.m0,
                Rect::from_spans(target.hspan().union(via.brect().hspan()), target.vspan()),
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
            if side == Side::Left {
                split_via1.push(viap);
            }
            blbr_stripes.push(stripe.vspan());
            ctx.draw_ref(&via)?;

            ctx.draw_rect(pc.h_metal, stripe);
            if let Some(name) = name {
                ctx.add_port(CellPort::with_shape(name, pc.h_metal, stripe))
                    .unwrap();
            }
        }

        let mut vdd_stripe =
            Span::new(blbr_stripes[1].stop(), blbr_stripes[0].start()).shrink_all(300);
        if vdd_stripe.length() > 1800 {
            vdd_stripe = Span::from_center_span_gridded(vdd_stripe.center(), 1800, 5);
        }
        let mut vss_stripe =
            Span::new(blbr_stripes[3].stop(), blbr_stripes[2].start()).shrink_all(300);
        if vss_stripe.length() > 1800 {
            vss_stripe = Span::from_center_span_gridded(vss_stripe.center(), 1800, 5);
        }
        if vss_stripe.length() < 200 || vdd_stripe.length() < 200 {
            panic!("insufficient space for vdd/vss stripe");
        }

        metadata.vdd_stripe(vdd_stripe);
        metadata.vss_stripe(vss_stripe);
        metadata.blbr_stripes(blbr_stripes);
        metadata.split_via0(split_via0);
        metadata.split_via1(split_via1);
        metadata.split_track(tracks[0]);
        metadata.nwell_vspan(pmos.brect().vspan());

        ctx.set_metadata(metadata.build().unwrap());

        assert!(self.params.idx < self.params.mux_ratio);

        for (inst, port) in [(&pmos, "sel_b"), (&nmos, "sel")] {
            for i in 0..self.params.mux_ratio {
                let vspan = if port == "sel" {
                    Span::with_start_and_length(
                        nmos.brect().top() + (line + space) * i as i64,
                        line,
                    )
                } else {
                    Span::with_stop_and_length(-(line + space) * i as i64, line)
                };
                let rect = Rect::from_spans(stripe_hspan, vspan);
                ctx.draw_rect(pc.h_metal, rect);
                ctx.add_port(CellPort::with_shape(PortId::new(port, i), pc.h_metal, rect))?;

                if i == self.params.idx {
                    let target = inst.port("gate_0")?.largest_rect(pc.m0)?;
                    let gate_conn =
                        Rect::from_spans(target.hspan(), target.vspan().union(rect.vspan()));

                    let via_rect = Rect::from_spans(pc.out_tracks.index(1), rect.vspan());

                    draw_via(pc.v_metal, via_rect, pc.h_metal, rect, ctx)?;

                    draw_via(pc.m0, gate_conn, pc.v_metal, via_rect, ctx)?;

                    ctx.draw_rect(pc.m0, gate_conn);
                }
            }
        }

        let power_stripe = Rect::from_spans(stripe_hspan, vdd_stripe);
        ctx.draw_rect(pc.h_metal, power_stripe);
        let ground_stripe = Rect::from_spans(stripe_hspan, vss_stripe);
        ctx.draw_rect(pc.h_metal, ground_stripe);

        let bounds = Rect::from_spans(Span::new(0, pc.width), ctx.brect().vspan());
        ctx.flatten();
        ctx.trim(&bounds);

        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        ctx.draw_rect(
            nwell,
            Rect::from_spans(bounds.hspan(), pmos.bbox().into_rect().vspan()),
        );

        let psdm = layers.get(Selector::Name("psdm"))?;
        let implants = ctx
            .elems()
            .filter(|elem| elem.layer.layer() == psdm)
            .map(|elem| elem.brect().vspan())
            .collect::<Vec<_>>();
        for span in implants {
            ctx.draw_rect(psdm, Rect::from_spans(bounds.hspan(), span));
        }

        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let implants = ctx
            .elems()
            .filter(|elem| elem.layer.layer() == nsdm)
            .map(|elem| elem.brect().vspan())
            .collect::<Vec<_>>();
        for span in implants {
            ctx.draw_rect(nsdm, Rect::from_spans(bounds.hspan(), span));
        }

        let outline = layers.get(Selector::Name("outline"))?;
        ctx.draw_rect(
            outline,
            ctx.brect()
                .expand_side(Side::Bot, 2 * (GATE_LINE + GATE_SPACE)),
        );

        Ok(())
    }
}

#[derive(Debug, Builder)]
struct Metadata {
    bl_in_top: i64,
    split_via1: Vec<ViaParams>,
    split_via0: Vec<ViaParams>,
    split_track: Rect,
    blbr_stripes: Vec<Span>,
    nwell_vspan: Span,
    sel_tracks_ystart: i64,
    vdd_stripe: Span,
    vss_stripe: Span,
}

impl Metadata {
    pub fn builder() -> MetadataBuilder {
        MetadataBuilder::default()
    }
}

impl TGateMuxCent {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        tgate_mux_tap_layout(pc.tap_width, false, &self.params, ctx)?;
        Ok(())
    }
}

fn tgate_mux_tap_layout(
    width: i64,
    end: bool,
    params: &TGateMuxParams,
    ctx: &mut LayoutCtx,
) -> substrate::error::Result<()> {
    let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

    let mux = ctx.instantiate::<TGateMux>(params)?;
    let stripe_hspan = Span::new(-width, 2 * width);

    let meta = mux.cell().get_metadata::<Metadata>();

    for via in meta.split_via0.iter().chain(meta.split_via1.iter()) {
        let mut via = ctx.instantiate::<Via>(via)?;
        via.place_center(Point::new(width, via.brect().center().y));
        ctx.draw_ref(&via)?;
        if !end {
            via.place_center(Point::new(0, via.brect().center().y));
            ctx.draw_ref(&via)?;
        }
    }

    let mut vtrack = meta.split_track.double(Side::Left);
    // if !end {
    //     ctx.draw_rect(pc.v_metal, vtrack);
    // }
    vtrack.place_center(Point::new(width, vtrack.center().y));
    // ctx.draw_rect(pc.v_metal, vtrack);

    let line = params.sel_width;
    let space = std::cmp::max(line / 4, 140);
    for i in 0..params.mux_ratio {
        let vspan = Span::with_stop_and_length(-(line + space) * i as i64, line);
        let rect = Rect::from_spans(stripe_hspan, vspan);
        ctx.draw_rect(pc.h_metal, rect);
        ctx.add_port(CellPort::with_shape(
            PortId::new("sel_b", i),
            pc.h_metal,
            rect,
        ))?;
    }
    for i in 0..params.mux_ratio {
        let vspan =
            Span::with_start_and_length(meta.sel_tracks_ystart + (line + space) * i as i64, line);
        let rect = Rect::from_spans(stripe_hspan, vspan);
        ctx.draw_rect(pc.h_metal, rect);
        ctx.add_port(CellPort::with_shape(
            PortId::new("sel", i),
            pc.h_metal,
            rect,
        ))?;
    }

    let power_stripe = Rect::from_spans(stripe_hspan, meta.vdd_stripe);
    ctx.draw_rect(pc.h_metal, power_stripe);
    ctx.add_port(CellPort::with_shape("vdd", pc.h_metal, power_stripe))
        .unwrap();
    let ground_stripe = Rect::from_spans(stripe_hspan, meta.vss_stripe);
    ctx.draw_rect(pc.h_metal, ground_stripe);
    ctx.add_port(CellPort::with_shape("vss", pc.h_metal, ground_stripe))
        .unwrap();

    let bounds = Rect::from_spans(Span::new(0, width), mux.brect().vspan());
    ctx.flatten();
    ctx.trim(&bounds);

    let ntap_bounds = Rect::from_spans(bounds.hspan(), meta.nwell_vspan);
    let tap_rect = ntap_bounds.shrink(300);

    let layers = ctx.layers();
    let taplayer = layers.get(Selector::Name("tap"))?;
    let nwell = layers.get(Selector::Name("nwell"))?;
    let nsdm = layers.get(Selector::Name("nsdm"))?;
    let psdm = layers.get(Selector::Name("psdm"))?;
    let outline = layers.get(Selector::Name("outline"))?;

    // Draw n+ tap to nwell.
    let viap = ViaParams::builder()
        .layers(taplayer, pc.m0)
        .geometry(tap_rect, tap_rect)
        .build();
    let tap = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&tap)?;

    let target = tap.layer_bbox(pc.m0).into_rect();
    let viap = ViaParams::builder()
        .layers(pc.m0, pc.v_metal)
        .geometry(target, target)
        .build();
    let via = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&via)?;

    let target = via.layer_bbox(pc.v_metal).into_rect();
    let viap = ViaParams::builder()
        .layers(pc.v_metal, pc.h_metal)
        .geometry(target, power_stripe)
        .build();
    let via = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&via)?;

    // Draw p+ tap to substrate.
    let ptap_bounds = Rect::from_spans(
        bounds.hspan().shrink_all(300),
        Span::new(meta.nwell_vspan.stop(), bounds.vspan().stop()).shrink_all(600),
    );
    let viap = ViaParams::builder()
        .layers(taplayer, pc.m0)
        .geometry(ptap_bounds, ptap_bounds)
        .build();
    let tap = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&tap)?;
    let psdm_rect = tap.layer_bbox(taplayer).into_rect().expand(130);
    ctx.draw_rect(psdm, psdm_rect);

    let target = tap.layer_bbox(pc.m0).into_rect();
    let viap = ViaParams::builder()
        .layers(pc.m0, pc.v_metal)
        .geometry(target, target)
        .build();
    let via = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&via)?;

    let target = via.layer_bbox(pc.v_metal).into_rect();
    let viap = ViaParams::builder()
        .layers(pc.v_metal, pc.h_metal)
        .geometry(target, ground_stripe)
        .build();
    let via = ctx.instantiate::<Via>(&viap)?;
    ctx.draw_ref(&via)?;

    for &stripe in meta.blbr_stripes.iter() {
        if !end {
            ctx.draw_rect(pc.h_metal, Rect::from_spans(Span::new(0, 200), stripe));
        }
        ctx.draw_rect(
            pc.h_metal,
            Rect::from_spans(Span::with_stop_and_length(width, 200), stripe),
        );
    }

    ctx.draw_rect(nwell, ntap_bounds);
    ctx.draw_rect(nsdm, ntap_bounds);

    let mux_outline = mux.layer_bbox(outline).into_rect();
    ctx.draw_rect(
        outline,
        Rect::from_spans(ctx.brect().hspan(), mux_outline.vspan()),
    );

    Ok(())
}

impl TGateMuxEnd {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;
        tgate_mux_tap_layout(pc.tap_width, true, &self.params, ctx)?;
        Ok(())
    }
}

impl TGateMuxGroup {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let params = TGateMuxParams {
            idx: 0,
            ..self.params
        };
        let gate = ctx.instantiate::<TGateMux>(&params)?;
        let meta = gate.cell().get_metadata::<Metadata>();
        let mut tiler = ArrayTiler::builder();

        for i in 0..self.params.mux_ratio {
            let params = TGateMuxParams {
                idx: i,
                ..self.params
            };
            let mut gate = ctx.instantiate::<TGateMux>(&params)?;
            if i % 2 != 0 {
                gate.reflect_horiz_anchored();
            }
            tiler.push(gate);
        }
        let mut tiler = tiler
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::Bottom)
            .build();
        tiler.expose_ports(
            |port: CellPort, i| match port.name().as_str() {
                "sel" | "sel_b" => Some(port),
                "bl_out" | "br_out" => None,
                "bl" | "br" => Some(port.with_index(i)),
                _ => Some(port),
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();
        let blbr_top = meta.bl_in_top + tiler.translation(0).y;

        ctx.draw_ref(&tiler)?;

        for i in 0..2 {
            let brm1 = Rect::from_spans(
                Span::from_center_span((self.params.mux_ratio / 2 + i) as i64 * pc.width, 280),
                Span::new(0, blbr_top),
            );
            ctx.draw_rect(pc.v_metal, brm1);
            let brm2 = Rect::from_spans(
                Span::new(0, self.params.mux_ratio as i64 * pc.width),
                Span::with_start_and_length((1 - i as i64) * (GATE_LINE + GATE_SPACE), GATE_LINE),
            );
            ctx.draw_rect(pc.h_metal, brm2);

            let viap = ViaParams::builder()
                .layers(pc.v_metal, pc.h_metal)
                .geometry(brm1, brm2)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
            let port_name = if i == 0 { "br_out" } else { "bl_out" };

            ctx.add_port(CellPort::with_shape(port_name, pc.h_metal, brm2))?;
        }

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let mux_outline = gate.layer_bbox(outline).into_rect();
        ctx.draw_rect(
            outline,
            Rect::from_spans(
                ctx.brect().hspan(),
                mux_outline.vspan().translate(tiler.translation(0).y),
            ),
        );

        Ok(())
    }
}
