use serde::Serialize;
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::transform::Translate;
use subgeom::{Dir, Point, Rect, Shape, Side, Sign, Span};
use substrate::component::Component;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, MustConnect, Port, PortConflictStrategy, PortId};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::{LayerBoundBox, LayerKey};
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::SimpleJog;
use substrate::layout::routing::tracks::{
    Boundary, CenteredTrackParams, FixedTracks, UniformTracks,
};
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};
use substrate::script::Script;

use super::{Precharge, PrechargeParams};

/// Precharge taps.
pub struct PrechargeCent {
    params: PrechargeParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrechargeEndParams {
    pub via_top: bool,
    pub inner: PrechargeParams,
}

/// Precharge end cap.
pub struct PrechargeEnd {
    params: PrechargeEndParams,
}

/// Single replica precharge with taps.
pub struct ReplicaPrecharge {
    params: ReplicaPrechargeParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplicaPrechargeParams {
    pub cols: usize,
    pub inner: PrechargeParams,
}

const LI_VIA_SHRINK: i64 = 20;

impl Precharge {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<PhysicalDesignScript>(&self.params)?;
        let db = ctx.mos_db();
        let mos = db
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())
            .unwrap();
        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 3],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::BothSides,
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
        let gate_shapes =
            [dsn.gate_stripe_bot, dsn.gate_stripe_top].map(|s| Rect::from_spans(stripe_span, s));
        gate_shapes
            .iter()
            .for_each(|r| ctx.draw_rect(dsn.h_metal, *r));
        ctx.add_port(
            CellPort::with_shapes("en_b", dsn.h_metal, gate_shapes.map(Shape::Rect))
                .must_connect(MustConnect::Yes),
        )?;

        let mut mos = ctx.instantiate::<LayoutMos>(&params)?;
        mos.set_orientation(Named::R90);

        mos.place_center(Point::new(dsn.width / 2, 0));
        mos.align_above(gate_shapes[0], 0);
        ctx.draw_ref(&mos)?;

        let cut = dsn.cut_bot;
        let gate = mos.port("gate_0")?.largest_rect(dsn.m0)?;

        let mut orects = Vec::with_capacity(dsn.out_tracks.len());
        for i in 0..dsn.out_tracks.len() {
            let top = if i == 1 { gate.top() } else { cut };
            let rect = Rect::from_spans(
                dsn.out_tracks.index(i),
                Span::new(std::cmp::min(0, dsn.gate_stripe_bot.start()), top),
            );
            if i != 1 {
                ctx.draw_rect(dsn.v_metal, rect);
            }
            orects.push(rect);
            if i == 0 {
                ctx.add_port(CellPort::with_shape("br_out", dsn.v_metal, rect))
                    .unwrap();
            }
            if i == 2 {
                ctx.add_port(CellPort::with_shape("bl_out", dsn.v_metal, rect))
                    .unwrap();
            }
        }
        let gate_x = mos.port("gate_0_x")?.largest_rect(dsn.m0)?;
        let gate_x = Rect::from_spans(
            dsn.out_tracks.index(1),
            Span::new(gate_x.bottom(), ctx.brect().top()),
        );

        let jog = SimpleJog::builder()
            .dir(Dir::Vert)
            .src_pos(dsn.cut_bot)
            .src([dsn.out_tracks.index(0), dsn.out_tracks.index(2)])
            .dst([dsn.in_tracks.index(1), dsn.in_tracks.index(2)])
            .line(dsn.v_line)
            .space(dsn.v_space)
            .layer(dsn.v_metal)
            .build()
            .unwrap();

        let mut rects = vec![];
        for i in 0..dsn.in_tracks.len() {
            let mut rect = Rect::from_spans(
                dsn.in_tracks.index(i),
                Span::new(jog.dst_pos(), dsn.cut_top + 40),
            );
            rects.push(rect);
            if i == 0 || i == 3 {
                rect = rect.expand_dir(Dir::Horiz, 60);
            }
            ctx.draw_rect(dsn.v_metal, rect);
        }

        ctx.draw(jog)?;

        let mut via0 = ViaParams::builder()
            .layers(dsn.m0, dsn.v_metal)
            .geometry(
                mos.port("sd_1_0")?
                    .largest_rect(dsn.m0)?
                    .expand_dir(Dir::Vert, -LI_VIA_SHRINK),
                rects[2],
            )
            .expand(ViaExpansion::LongerDirection)
            .build();

        let mut via = ctx.instantiate::<Via>(&via0)?;
        // HACK to get the sd_1_0 and sd_2_1 vias to be symmetric w.r.t the transistor drain.
        via.translate(Point::new(5, 0));
        ctx.draw(via)?;

        let target = mos
            .port("sd_2_1")?
            .largest_rect(dsn.m0)?
            .expand_dir(Dir::Vert, -LI_VIA_SHRINK);
        via0.set_geometry(target, rects[1]);
        let via = ctx.instantiate::<Via>(&via0)?;
        ctx.draw(via)?;

        let mut m1_vias = Vec::with_capacity(2);
        for (port, rect, x) in [("sd_2_0", rects[3], dsn.width), ("sd_1_1", rects[0], 0)] {
            let port = mos.port(port)?.largest_rect(dsn.m0)?;
            via0.set_geometry(
                Rect::from_spans(Span::from_point(x), port.vspan().shrink_all(LI_VIA_SHRINK)),
                rect,
            );
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
                Rect::from_spans(
                    Span::from_point(rect.center().x),
                    port.vspan().shrink_all(LI_VIA_SHRINK),
                ),
                rect.expand_dir(Dir::Vert, -4 * LI_VIA_SHRINK),
            );
            let via = ctx.instantiate::<Via>(&via0)?;
            ctx.draw(via)?;
        }

        via0.set_geometry(
            gate_shapes[0],
            orects[1].expand_dir(Dir::Vert, -4 * LI_VIA_SHRINK),
        );
        let via = ctx.instantiate::<Via>(&via0)?;
        ctx.draw(via)?;

        via0.set_geometry(
            gate_shapes[1],
            gate_x.expand_dir(Dir::Vert, -4 * LI_VIA_SHRINK),
        );
        let via = ctx.instantiate::<Via>(&via0)?;
        ctx.draw(via)?;

        let stripe = Rect::from_spans(stripe_span, dsn.power_stripe);
        ctx.draw_rect(dsn.h_metal, stripe);
        ctx.add_port(CellPort::with_shape("vdd", dsn.h_metal, stripe))
            .unwrap();

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

        via1.set_geometry(orects[1], gate_shapes[0]);
        let via = ctx.instantiate::<Via>(&via1)?;
        ctx.draw(via)?;

        via1.set_geometry(gate_x, gate_shapes[1]);
        let via = ctx.instantiate::<Via>(&via1)?;
        let gate_ct_top = via.brect().top();
        ctx.draw(via)?;

        ctx.draw_rect(dsn.m0, Rect::from_spans(gate.hspan(), orects[1].vspan()));
        ctx.draw_rect(dsn.m0, Rect::from_spans(gate.hspan(), gate_x.vspan()));

        let jog = SimpleJog::builder()
            .dir(Dir::Vert)
            .src_pos(dsn.cut_top + 40)
            .src([dsn.in_tracks.index(1), dsn.in_tracks.index(2)])
            .dst([dsn.out_tracks.index(0), dsn.out_tracks.index(2)])
            .line(dsn.v_line + 40)
            .space(dsn.v_space + 40)
            .layer(dsn.v_metal)
            .build()
            .unwrap();
        for i in [0, 2] {
            ctx.draw_rect(
                dsn.v_metal,
                Rect::from_spans(
                    dsn.out_tracks.index(i),
                    Span::new(jog.dst_pos(), gate_ct_top),
                ),
            );
        }
        ctx.draw(jog)?;

        let jog = SimpleJog::builder()
            .dir(Dir::Vert)
            .src_pos(gate_ct_top)
            .src([dsn.out_tracks.index(0), dsn.out_tracks.index(2)])
            .dst([dsn.in_tracks.index(1), dsn.in_tracks.index(2)])
            .line(dsn.v_line + 40)
            .space(dsn.v_space + 40)
            .layer(dsn.v_metal)
            .build()
            .unwrap();
        let rect = Rect::from_spans(
            dsn.in_tracks.index(1),
            Span::with_stop_and_length(jog.dst_pos(), dsn.v_line),
        );
        ctx.add_port(CellPort::with_shape("br_in", dsn.v_metal, rect))?;
        let rect = Rect::from_spans(
            dsn.in_tracks.index(2),
            Span::with_stop_and_length(jog.dst_pos(), dsn.v_line),
        );
        ctx.add_port(CellPort::with_shape("bl_in", dsn.v_metal, rect))?;
        ctx.draw(jog)?;

        ctx.flatten();

        let bounds = ctx.brect().with_hspan(Span::new(0, dsn.width));

        let layers = ctx.layers();
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let implants = ctx
            .elems()
            .filter(|elem| elem.layer.layer() == psdm)
            .map(|elem| elem.brect().vspan())
            .collect::<Vec<_>>();
        for span in implants {
            ctx.draw_rect(psdm, Rect::from_spans(bounds.hspan(), span));
        }
        ctx.draw_rect(nwell, bounds);
        ctx.trim(&bounds);
        Ok(())
    }
}

struct Metadata {
    m1_via_bot: ViaParams,
    m1_via_top: ViaParams,
    m2_via: ViaParams,
}

impl Component for PrechargeCent {
    type Params = PrechargeParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("precharge_cent")
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
        let pc = ctx.instantiate::<Precharge>(&self.params)?;
        let dsn = ctx
            .inner()
            .run_script::<PhysicalDesignScript>(&self.params)?;
        let meta = pc.cell().get_metadata::<Metadata>();
        let layers = ctx.layers();

        let tap = layers.get(Selector::Name("tap"))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let m0 = layers.get(Selector::Metal(0))?;

        let brect = Rect::new(
            Point::new(0, pc.brect().bottom()),
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

        let y = dsn.cut_bot + 2 * dsn.v_line + dsn.v_space;
        let half_tr = Rect::from_spans(
            Span::new(0, dsn.v_line / 2 + 60),
            Span::new(y, dsn.cut_top + 40),
        );
        ctx.draw_rect(dsn.v_metal, half_tr);

        let mut via = ctx.instantiate::<Via>(&meta.m1_via_top)?;
        via.place_center(Point::new(0, via.brect().center().y));
        ctx.draw(via.clone())?;
        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(0, tap.layer_bbox(dsn.m0).p1.x),
                via.brect().vspan(),
            ),
        );

        via.place_center(Point::new(dsn.tap_width, via.brect().center().y));
        ctx.draw(via.clone())?;

        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(tap.layer_bbox(dsn.m0).p0.x, dsn.tap_width),
                via.brect().vspan(),
            ),
        );

        let half_tr = Rect::from_spans(
            Span::with_stop_and_length(dsn.tap_width, dsn.v_line / 2 + 60),
            Span::new(y, dsn.cut_top + 40),
        );
        ctx.draw_rect(dsn.v_metal, half_tr);

        let stripe_span = Span::new(-dsn.tap_width, 2 * dsn.tap_width);
        let shapes =
            [dsn.gate_stripe_bot, dsn.gate_stripe_top].map(|s| Rect::from_spans(stripe_span, s));
        shapes.iter().for_each(|r| ctx.draw_rect(dsn.h_metal, *r));
        ctx.add_port(CellPort::with_shapes(
            "en_b",
            dsn.h_metal,
            shapes.map(Shape::Rect),
        ))?;

        let power_stripe = Rect::from_spans(stripe_span, dsn.power_stripe);
        ctx.draw_rect(dsn.h_metal, power_stripe);
        ctx.add_port(CellPort::with_shape("vdd", dsn.h_metal, power_stripe))
            .unwrap();

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

impl Component for PrechargeEnd {
    type Params = PrechargeEndParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("precharge_end")
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
        let pc = ctx.instantiate::<Precharge>(&self.params.inner)?;
        let dsn = ctx
            .inner()
            .run_script::<PhysicalDesignScript>(&self.params.inner)?;
        let meta = pc.cell().get_metadata::<Metadata>();
        let layers = ctx.layers();

        let tap = layers.get(Selector::Name("tap"))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let m0 = layers.get(Selector::Metal(0))?;

        let brect = Rect::new(
            Point::new(0, pc.brect().bottom()),
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

        let y = dsn.cut_bot + 2 * dsn.v_line + dsn.v_space;

        let mut via = ctx.instantiate::<Via>(if self.params.via_top {
            &meta.m1_via_top
        } else {
            &meta.m1_via_bot
        })?;
        via.place_center(Point::new(dsn.tap_width, via.brect().center().y));
        ctx.draw(via.clone())?;
        ctx.draw_rect(
            dsn.m0,
            Rect::from_spans(
                Span::new(tap.layer_bbox(dsn.m0).p0.x, dsn.tap_width),
                via.brect().vspan(),
            ),
        );

        let half_tr = Rect::from_spans(
            Span::with_stop_and_length(dsn.tap_width, dsn.v_line / 2 + 60),
            Span::new(y, dsn.cut_top + 40),
        );
        ctx.draw_rect(dsn.v_metal, half_tr);

        let stripe_span = Span::new(-dsn.tap_width, 2 * dsn.tap_width);
        let shapes =
            [dsn.gate_stripe_bot, dsn.gate_stripe_top].map(|s| Rect::from_spans(stripe_span, s));
        shapes.iter().for_each(|r| ctx.draw_rect(dsn.h_metal, *r));
        ctx.add_port(CellPort::with_shapes(
            "en_b",
            dsn.h_metal,
            shapes.map(Shape::Rect),
        ))?;

        let power_stripe = Rect::from_spans(stripe_span, dsn.power_stripe);
        ctx.draw_rect(dsn.h_metal, power_stripe);
        ctx.add_port(CellPort::with_shape("vdd", dsn.h_metal, power_stripe))
            .unwrap();

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

impl Component for ReplicaPrecharge {
    type Params = ReplicaPrechargeParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("replica_precharge")
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
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let grid = ctx.pdk().layout_grid();

        let via12 = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m1, m2)
                .geometry(
                    Rect::from_point(Point::zero()),
                    Rect::from_point(Point::zero()),
                )
                .bot_extension(Dir::Vert)
                .top_extension(Dir::Vert)
                .build(),
        )?;

        let pc = ctx.instantiate::<Precharge>(&self.params.inner)?;
        let mut pc_end_top = ctx.instantiate::<PrechargeEnd>(&PrechargeEndParams {
            via_top: true,
            inner: self.params.inner,
        })?;
        pc_end_top.set_orientation(Named::ReflectHoriz);
        let pc_end_bot = ctx.instantiate::<PrechargeEnd>(&PrechargeEndParams {
            via_top: false,
            inner: self.params.inner,
        })?;

        let mut tiler = ArrayTiler::builder();

        tiler.push(pc_end_bot.clone());

        for i in 0..self.params.cols {
            if i % 2 == 0 {
                tiler.push(pc.clone());
            } else {
                tiler.push(pc.with_orientation(Named::ReflectHoriz));
            }
        }

        let mut tiler = tiler
            .push(if self.params.cols % 2 == 0 {
                pc_end_bot.with_orientation(Named::ReflectHoriz)
            } else {
                pc_end_top
            })
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::CenterVertical)
            .build();

        tiler.expose_ports(
            |port: CellPort, i| match port.name().as_str() {
                "bl_in" | "br_in" | "bl_out" | "br_out" => Some(port.with_index(i - 1)),
                _ => Some(port),
            },
            PortConflictStrategy::Merge,
        )?;

        let en_b_rect = tiler.port_map().port("en_b")?.largest_rect(m2)?;
        let tracks = UniformTracks::builder()
            .line(en_b_rect.height())
            .space(140)
            .start(en_b_rect.top())
            .sign(Sign::Neg)
            .build()
            .unwrap();

        for (i, (port_name, out_port)) in [("bl_out", "rbl"), ("br_out", "rbr")].iter().enumerate()
        {
            let track_span = tracks.index(i + 1);
            ctx.draw_rect(m2, Rect::from_spans(en_b_rect.hspan(), track_span));
            ctx.merge_port(CellPort::with_shape(
                *out_port,
                m2,
                Rect::from_spans(en_b_rect.hspan(), track_span),
            ));
            for j in 0..2 {
                let port_rect = tiler
                    .port_map()
                    .port(PortId::new(*port_name, j))?
                    .largest_rect(m1)?;
                ctx.draw_rect(
                    m1,
                    port_rect.with_vspan(port_rect.vspan().union(track_span)),
                );
                let mut via = via12.clone();
                via.align_centers_gridded(Rect::from_spans(port_rect.hspan(), track_span), grid);
                ctx.draw(via)?;
            }
        }

        ctx.add_ports(tiler.ports().cloned())?;
        ctx.draw(tiler)?;

        Ok(())
    }
}

pub struct PhysicalDesignScript;

pub struct PhysicalDesign {
    /// Location of the horizontal power strap
    pub(crate) power_stripe: Span,
    pub(crate) gate_stripe_bot: Span,
    pub(crate) gate_stripe_top: Span,
    pub(crate) h_metal: LayerKey,
    pub(crate) cut_bot: i64,
    pub(crate) cut_top: i64,
    pub(crate) width: i64,
    pub(crate) in_tracks: FixedTracks,
    pub(crate) out_tracks: FixedTracks,
    pub(crate) v_metal: LayerKey,
    pub(crate) v_line: i64,
    pub(crate) v_space: i64,
    pub(crate) m0: LayerKey,
    pub(crate) tap_width: i64,
}

impl Script for PhysicalDesignScript {
    type Params = PrechargeParams;
    type Output = PhysicalDesign;

    fn run(
        params: &Self::Params,
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

        let power_stripe_height = params.pull_up_width.clamp(800, 3_600);
        let power_stripe = Span::with_start_and_length(
            params.equalizer_width + params.pull_up_width + 900 + 270 - 60 + 270 / 2
                - power_stripe_height / 2,
            power_stripe_height,
        );
        let gate_stripe_bot = Span::with_stop_and_length(360, params.en_b_width);
        let gate_stripe_top = Span::with_start_and_length(
            params.equalizer_width + 2 * params.pull_up_width + 2_340 - 360,
            params.en_b_width,
        );
        let cut_bot = 815 - 60 + params.equalizer_width;
        let cut_top = params.equalizer_width + 2 * params.pull_up_width + 1_380;

        Ok(PhysicalDesign {
            power_stripe,
            gate_stripe_bot,
            gate_stripe_top,
            h_metal: m2,
            cut_bot,
            cut_top,
            width: 1_200,
            v_metal: m1,
            v_line: 140,
            v_space: 140,
            in_tracks,
            out_tracks,
            tap_width: 1_300,
            m0,
        })
    }
}
