use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::transform::Translate;
use subgeom::{Dir, Point, Rect, Shape, Side, Sign, Span};
use substrate::component::{Component, NoParams};
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Instance, Port, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::{LayerBoundBox, LayerKey};
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::placement::tile::LayerBbox;
use substrate::layout::routing::auto::straps::{RoutedStraps, Target};
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::{OffsetJog, SimpleJog};
use substrate::layout::routing::tracks::{TrackLocator, UniformTracks};
use substrate::layout::straps::SingleSupplyNet;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::mos::SchematicMos;

use crate::blocks::bitcell_array::replica::ReplicaCellArray;
use crate::blocks::bitcell_array::SpCellArray;
use crate::blocks::columns::layout::DffArray;
use crate::blocks::columns::{ColPeripherals, ColumnDesignScript};
use crate::blocks::control::ControlLogicReplicaV2;
use crate::blocks::decoder::{Decoder, DecoderStage};
use crate::blocks::precharge;
use crate::blocks::precharge::layout::{ReplicaPrecharge, LI_VIA_SHRINK};

use super::{SramInner, SramPhysicalDesignScript, TappedDiode};

/// Returns the layer used for routing in the provided direction.
///
/// The SRAM top level only uses m1 and m2 for vertical and horizontal routing, respectively.
fn get_layer(dir: Dir, ctx: &LayoutCtx) -> Result<LayerKey> {
    ctx.layers().get(Selector::Metal(match dir {
        Dir::Horiz => 2,
        Dir::Vert => 1,
    }))
}

// Draw rect and block it in router.
fn draw_rect(layer: LayerKey, rect: Rect, router: &mut GreedyRouter, ctx: &mut LayoutCtx) {
    ctx.draw_rect(layer, rect);
    router.block(layer, rect);
}

// Draw via between two rects.
//
// Bottom layer must be provided first.
pub(crate) fn draw_via(
    layer1: LayerKey,
    rect1: Rect,
    layer2: LayerKey,
    rect2: Rect,
    ctx: &mut LayoutCtx,
) -> Result<Instance> {
    let via = ctx.instantiate::<Via>(
        &ViaParams::builder()
            .layers(layer1, layer2)
            .geometry(rect1, rect2)
            .expand(ViaExpansion::LongerDirection)
            .build(),
    )?;
    ctx.draw_ref(&via)?;
    Ok(via)
}

// Draw via between two rects on routing layers.
//
// Automatically determines top and bottom layers.
fn draw_routing_via(
    layer1: LayerKey,
    rect1: Rect,
    layer2: LayerKey,
    rect2: Rect,
    ctx: &mut LayoutCtx,
) -> Result<Instance> {
    let (layer1, rect1, layer2, rect2) = if layer1 == get_layer(Dir::Vert, ctx)? {
        (layer1, rect1, layer2, rect2)
    } else {
        (layer2, rect2, layer1, rect1)
    };
    let via = ctx.instantiate::<Via>(
        &ViaParams::builder()
            .layers(layer1, layer2)
            .geometry(rect1, rect2)
            .build(),
    )?;
    ctx.draw_ref(&via)?;
    Ok(via)
}

/// Draws a route from `start` to `end` using the provided `tracks`.
/// `start` is the beginning rect, which may or may not be grid aligned.
/// `end` is the ending rect, which may or may not be grid aligned.
/// `dir` is the direction of the first provided track.
/// `tracks` are the tracks along which the route should be drawn.
/// Expands `start` and `end` to contain the adjacent track, adding a via if necessary.
fn draw_route(
    start_layer: LayerKey,
    start: Rect,
    end_layer: LayerKey,
    end: Rect,
    dir: Dir,
    tracks: Vec<i64>,
    router: &mut GreedyRouter,
    ctx: &mut LayoutCtx,
) -> Result<()> {
    assert!(
        !tracks.is_empty(),
        "must provide at least one routing track"
    );

    let mut spans = vec![start.span(dir)];
    spans.extend(tracks.iter().enumerate().map(|(i, track)| {
        router
            .track_info(get_layer(if i % 2 == 0 { dir } else { !dir }, ctx).unwrap())
            .tracks()
            .index(*track)
    }));
    let end_dir = if tracks.len() % 2 == 0 { dir } else { !dir };
    spans.push(end.span(!end_dir));
    let mut curr_dir = dir;
    let mut prev_rect = None;

    // Expand `start` rect to connect to tracks.
    let expanded_start = start.with_span(start.span(!dir).union(spans[1]), !dir);
    draw_rect(start_layer, expanded_start, router, ctx);
    let next_layer = get_layer(dir, ctx)?;
    if start_layer != next_layer {
        prev_rect = Some(expanded_start);
    }

    for spans in spans.windows(3) {
        let (prev_track_span, curr_track_span, next_track_span) = (spans[0], spans[1], spans[2]);
        if prev_track_span.intersects(&next_track_span) {
            prev_rect = None;
        } else {
            let curr_layer = get_layer(curr_dir, ctx)?;
            let next_layer = get_layer(!curr_dir, ctx)?;
            let rect = Rect::span_builder()
                .with(curr_dir, prev_track_span.union(next_track_span))
                .with(!curr_dir, curr_track_span)
                .build();
            draw_rect(curr_layer, rect, router, ctx);
            if let Some(prev_rect) = prev_rect {
                draw_routing_via(next_layer, prev_rect, curr_layer, rect, ctx)?;
            }
            prev_rect = Some(rect);
        }
        curr_dir = !curr_dir;
    }

    let expanded_end = end.with_span(end.span(end_dir).union(spans[tracks.len()]), end_dir);
    draw_rect(end_layer, expanded_end, router, ctx);
    let prev_layer = get_layer(!curr_dir, ctx)?;
    if end_layer != prev_layer {
        if let Some(prev_rect) = prev_rect {
            draw_routing_via(prev_layer, prev_rect, end_layer, expanded_end, ctx)?;
        }
    }
    Ok(())
}

pub struct ColumnNmos {
    params: ColumnNmosParams,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColumnNmosParams {
    pub gate_width: i64,
    pub drain_width: i64,
    pub length: i64,
}

pub struct ColumnNmosCent {
    params: ColumnNmosParams,
}

/// Column NMOS replica to match replica bitline capacitance to fraction
/// of capacitance on main bitline.
pub struct ReplicaColumnNmos {
    params: ReplicaColumnNmosParams,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReplicaColumnNmosParams {
    pub cols: usize,
    pub inner: ColumnNmosParams,
}

impl Component for ColumnNmos {
    type Params = ColumnNmosParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column_nmos")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let vss = ctx.port("vss", Direction::InOut);
        let bl = ctx.port("bl", Direction::InOut);

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut gate_mos = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.gate_width,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        gate_mos.connect_all([("d", &vss), ("g", &bl), ("s", &vss), ("b", &vss)]);
        gate_mos.set_name("gate_mos");
        ctx.add_instance(gate_mos);

        let mut drain_mos = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.drain_width,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        drain_mos.connect_all([("d", &bl), ("g", &vss), ("s", &vss), ("b", &vss)]);
        drain_mos.set_name("drain_mos");
        ctx.add_instance(drain_mos);
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;
        let db = ctx.mos_db();
        let mos = db
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())
            .unwrap();
        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::Merge,
            devices: vec![MosParams {
                w: self.params.gate_width,
                l: self.params.length,
                m: 1,
                nf: 1,
                id: mos.id(),
            }],
        };
        let mut gate_mos = ctx.instantiate::<LayoutMos>(&params)?;
        gate_mos.set_orientation(Named::R90);
        gate_mos.place_center_x(dsn.width / 2);
        ctx.draw_rect(
            dsn.m0,
            gate_mos
                .port("sd_0_0")?
                .bbox(dsn.m0)
                .union(gate_mos.port("sd_0_1")?.bbox(dsn.m0))
                .into_rect(),
        );
        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::Merge,
            devices: vec![MosParams {
                w: self.params.drain_width,
                l: self.params.length,
                m: 1,
                nf: 1,
                id: mos.id(),
            }],
        };
        let mut drain_mos = ctx.instantiate::<LayoutMos>(&params)?;
        drain_mos.set_orientation(Named::R270);
        drain_mos.align_above(&gate_mos, 210);
        drain_mos.align_centers_horizontally_gridded(&gate_mos, ctx.pdk().layout_grid());
        let gate_rect = drain_mos.port("gate")?.largest_rect(dsn.m0)?;
        let jog = OffsetJog::builder()
            .src(drain_mos.port("sd_0_0")?.largest_rect(dsn.m0)?)
            .dst(gate_rect.right())
            .dir(Dir::Vert)
            .sign(Sign::Pos)
            .space(170)
            .layer(dsn.m0)
            .build()
            .unwrap();
        ctx.draw_rect(
            dsn.m0,
            gate_rect.with_vspan(gate_rect.vspan().union(jog.r2().vspan())),
        );
        ctx.draw(jog)?;

        let layers = ctx.layers();
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        for inst in [&gate_mos, &drain_mos] {
            ctx.draw_ref(inst)?;
        }
        ctx.draw_rect(
            nsdm,
            gate_mos
                .layer_bbox(nsdm)
                .union(drain_mos.layer_bbox(nsdm))
                .into_rect()
                .with_hspan(Span::from_center_span_gridded(
                    gate_mos.brect().center().x,
                    dsn.width,
                    ctx.pdk().layout_grid(),
                )),
        );

        let bl = Rect::from_spans(dsn.out_tracks.index(2), ctx.brect().vspan());
        ctx.add_port(CellPort::with_shape("bl", dsn.v_metal, bl))?;
        ctx.draw_rect(dsn.v_metal, bl);

        // Connect gate of gate NMOS and drain of drain NMOS to bitline.
        for (port, inst) in [("gate", &gate_mos), ("sd_0_1", &drain_mos)] {
            let m0_rect = inst.port(port)?.largest_rect(dsn.m0)?;
            let m0_rect = m0_rect.with_hspan(m0_rect.hspan().union(bl.hspan()));
            ctx.draw_rect(dsn.m0, m0_rect);
            draw_via(dsn.m0, m0_rect, dsn.v_metal, bl, ctx)?;
        }

        // Connect remaining sources to VSS.
        for (port, inst) in [("sd_0_1", &gate_mos), ("sd_0_0", &drain_mos)] {
            let m0_rect = inst.port(port)?.largest_rect(dsn.m0)?;
            let power_stripe_height = m0_rect.height().clamp(320, 3_600);
            let power_stripe = Rect::from_spans(
                ctx.brect().hspan(),
                Span::from_center_span_gridded(
                    m0_rect.center().y,
                    power_stripe_height,
                    ctx.pdk().layout_grid(),
                ),
            );
            ctx.merge_port(CellPort::with_shape("vss", dsn.h_metal, power_stripe));
            ctx.draw_rect(dsn.h_metal, power_stripe);
            draw_via(dsn.m0, m0_rect, dsn.v_metal, power_stripe, ctx)?;
            draw_via(dsn.v_metal, m0_rect, dsn.h_metal, power_stripe, ctx)?;
        }

        Ok(())
    }
}

impl Component for ColumnNmosCent {
    type Params = ColumnNmosParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column_nmos_end")
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
        let nmos = ctx.instantiate::<ColumnNmos>(&self.params)?;
        let dsn = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;
        let layers = ctx.layers();

        let tap = layers.get(Selector::Name("tap"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let brect = Rect::new(
            Point::new(0, nmos.brect().bottom()),
            Point::new(dsn.tap_width, nmos.brect().top()),
        );

        ctx.draw_rect(psdm, brect);

        let tap_rect = brect.shrink(300);

        let viap = ViaParams::builder()
            .layers(tap, m0)
            .geometry(tap_rect, tap_rect)
            .build();
        let tap = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_ref(&tap)?;

        for rect in nmos
            .port("vss")?
            .shapes(m2)
            .filter_map(|shape| shape.as_rect())
        {
            let rect = rect.with_hspan(brect.hspan());
            ctx.merge_port(CellPort::with_shape("vss", m2, rect));
            ctx.draw_rect(m2, rect);
            draw_via(m0, tap.layer_bbox(m0).brect(), m1, rect, ctx)?;
            draw_via(m1, tap.layer_bbox(m0).brect(), m2, rect, ctx)?;
        }

        Ok(())
    }
}

impl Component for ReplicaColumnNmos {
    type Params = ReplicaColumnNmosParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("replica_column_nmos")
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

        let pc = ctx.instantiate::<ColumnNmos>(&self.params.inner)?;
        let pc_cent = ctx.instantiate::<ColumnNmosCent>(&self.params.inner)?;

        let mut tiler = ArrayTiler::builder();

        tiler.push(pc_cent.clone());

        for i in 0..self.params.cols {
            if i % 2 == 0 {
                tiler.push(pc.clone());
            } else {
                tiler.push(pc.with_orientation(Named::ReflectHoriz));
            }
        }

        let mut tiler = tiler
            .push(pc_cent)
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::CenterVertical)
            .build();

        tiler.expose_ports(
            |port: CellPort, i| match port.name().as_str() {
                "bl" => Some(port.with_index(i - 1)),
                _ => Some(port),
            },
            PortConflictStrategy::Merge,
        )?;

        ctx.add_ports(tiler.ports().cloned())?;
        ctx.draw(tiler)?;

        Ok(())
    }
}

pub enum NeedsDiodes {
    Yes,
    No,
}

impl SramInner {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<SramPhysicalDesignScript>(&self.params)?;
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let bitcells = ctx.instantiate::<SpCellArray>(&dsn.bitcells)?;
        let mut cols = ctx.instantiate::<ColPeripherals>(&dsn.col_params)?;
        let mut decoder = ctx
            .instantiate::<Decoder>(&dsn.row_decoder)?
            .with_orientation(Named::R90Cw);
        let mut addr_gate = ctx
            .instantiate::<DecoderStage>(&dsn.addr_gate)?
            .with_orientation(Named::FlipYx);
        let mut col_dec = ctx
            .instantiate::<Decoder>(&dsn.col_decoder)?
            .with_orientation(Named::FlipYx);
        let mut control = ctx.instantiate::<ControlLogicReplicaV2>(&dsn.control)?;

        let mut pc_b_buffer = ctx
            .instantiate::<DecoderStage>(&dsn.pc_b_buffer)?
            .with_orientation(Named::R90Cw);
        let mut wlen_buffer = ctx.instantiate::<DecoderStage>(&dsn.wlen_buffer)?;
        let mut write_driver_en_buffer = ctx
            .instantiate::<DecoderStage>(&dsn.write_driver_en_buffer)?
            .with_orientation(Named::R90Cw);
        let mut sense_en_buffer = ctx
            .instantiate::<DecoderStage>(&dsn.sense_en_buffer)?
            .with_orientation(Named::R90Cw);
        let mut dffs = ctx.instantiate::<DffArray>(&dsn.num_dffs)?;
        let mut rbl = ctx.instantiate::<ReplicaCellArray>(&dsn.rbl)?;
        let mut replica_pc = ctx
            .instantiate::<ReplicaPrecharge>(&dsn.replica_pc)?
            .with_orientation(Named::ReflectVert);
        let mut replica_nmos = ctx
            .instantiate::<ReplicaColumnNmos>(&dsn.replica_nmos)?
            .with_orientation(Named::ReflectVert);

        // Align row decoders to left of bitcell array.
        decoder.align_to_the_left_of(bitcells.bbox(), 7_000);
        decoder.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());

        // Align wlen buffer to the left of row decoder.

        // Align wlen buffer and address gate to the left of the row decoder.
        //
        // Need enough vertical tracks to route outputs to decoder.
        addr_gate.align_to_the_left_of(
            decoder.bbox(),
            1_400 + 1_400 * self.params.row_bits() as i64,
        );
        wlen_buffer.align_right(addr_gate.bbox());
        wlen_buffer.translate(Point::new(-2_000, 0));
        wlen_buffer.align_bottom(decoder.bbox());
        addr_gate.align_above(wlen_buffer.bbox(), 2_000);

        // Align column peripherals under bitcell array.
        cols.align_beneath(bitcells.bbox(), 4_000);
        cols.align_centers_horizontally_gridded(bitcells.bbox(), ctx.pdk().layout_grid());

        // Align pc_b buffer with pc_b port of column peripherals.
        //
        // Need enough vertical tracks to route column decoder outputs to column mux, as well
        // as buffer outputs to column control signals (pc_b, sense_en, write_driver_en).
        //
        // Also add space for viaing to horizontal metal.
        pc_b_buffer.align_top(cols.bbox());
        pc_b_buffer.translate(Point::new(0, 1_000));
        pc_b_buffer.align_to_the_left_of(
            cols.bbox(),
            7_100
                + 700
                    * (2 * self.params.mux_ratio() as i64 * dsn.col_dec_routing_tracks
                        + dsn.sense_en_routing_tracks
                        + dsn.pc_b_routing_tracks
                        + dsn.write_driver_en_routing_tracks),
        );

        // Align sense_en buffer beneath pc_b buffer.
        sense_en_buffer.align_beneath(pc_b_buffer.brect(), 6_000);
        sense_en_buffer.align_right(pc_b_buffer.bbox());

        // Align column decoder beneath bottommost column mux select port.
        // This prevents overlapping of m2 routes.
        col_dec.align_beneath(
            cols.port("sense_en")?
                .largest_rect(m2)?
                .bbox()
                .union(sense_en_buffer.brect().expand_dir(Dir::Vert, 3_000).bbox()),
            3_000,
        );
        col_dec.align_right(pc_b_buffer.bbox());

        // Align write driver underneath column decoder.
        write_driver_en_buffer.align_beneath(col_dec.bbox(), 6_000);
        write_driver_en_buffer.align_right(pc_b_buffer.bbox());

        // Align control logic to the left of all of the buffers and column decoder that border
        // column peripherals.
        let buffer_bbox = col_dec
            .bbox()
            .union(sense_en_buffer.bbox())
            .union(pc_b_buffer.bbox())
            .union(write_driver_en_buffer.bbox())
            .into_rect();
        control.set_orientation(Named::R90);
        control.align_beneath(decoder.bbox(), 6_000);
        control.align_to_the_left_of(
            buffer_bbox,
            2_100 + 1_400 * self.params.col_select_bits() as i64,
        );

        // Align replica bitcell array to left of control logic, with replica precharge
        // aligned to top of control logic.
        rbl.align_to_the_left_of(control.bbox(), 7_000);
        replica_nmos.align_beneath(decoder.bbox(), 6_000);
        replica_nmos.align_centers_horizontally_gridded(rbl.bbox(), ctx.pdk().layout_grid());
        replica_pc.align_beneath(replica_nmos.bbox(), 0);
        replica_pc.align_centers_horizontally_gridded(rbl.bbox(), ctx.pdk().layout_grid());
        rbl.align_beneath(replica_pc.bbox(), 4_000);

        // Align DFFs to the left of column peripherals and underneath all other objects.
        dffs.align_right(pc_b_buffer.bbox());
        dffs.align_beneath(
            control
                .bbox()
                .union(rbl.bbox())
                .union(write_driver_en_buffer.bbox()),
            5_500 + 1_400 * self.params.addr_width() as i64,
        );

        // Draw instances.
        ctx.draw_ref(&bitcells)?;
        ctx.draw_ref(&cols)?;
        ctx.draw_ref(&decoder)?;
        ctx.draw_ref(&addr_gate)?;
        ctx.draw_ref(&wlen_buffer)?;
        ctx.draw_ref(&pc_b_buffer)?;
        ctx.draw_ref(&col_dec)?;
        ctx.draw_ref(&sense_en_buffer)?;
        ctx.draw_ref(&write_driver_en_buffer)?;
        ctx.draw_ref(&control)?;
        ctx.draw_ref(&dffs)?;
        ctx.draw_ref(&rbl)?;
        ctx.draw_ref(&replica_pc)?;
        ctx.draw_ref(&replica_nmos)?;

        // Set up autorouter for automatic strap placement.
        let router_bbox = ctx
            .brect()
            .expand(8 * 680)
            .expand_side(Side::Right, 4 * 680)
            .expand_side(Side::Left, 1_400 * self.params.row_bits() as i64)
            .snap_to_grid(680);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: router_bbox,
            layers: vec![
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Horiz,
                    layer: m2,
                },
            ],
        });
        let m1_tracks = router.track_info(m1).tracks().clone();
        let m2_tracks = router.track_info(m2).tracks().clone();

        // Block appropriate areas in router for each instance.
        for inst in [
            &addr_gate,
            &decoder,
            &col_dec,
            &pc_b_buffer,
            &wlen_buffer,
            &sense_en_buffer,
            &write_driver_en_buffer,
            &dffs,
            &control,
            &replica_pc,
            &replica_nmos,
        ] {
            for layer in [m1, m2] {
                for shape in inst.shapes_on(layer) {
                    let rect = shape.brect();
                    router.block(layer, rect);
                }
            }
        }

        // Block full m2 layer bbox of decoders due to wrong direction routing.
        // Also block m1 to the right of buffers and column decoder to prevent power strap placement in
        // between vias.
        for inst in [
            &decoder,
            &col_dec,
            &pc_b_buffer,
            &sense_en_buffer,
            &write_driver_en_buffer,
        ] {
            router.block(m2, inst.layer_bbox(m2).into_rect());
            router.block(m1, inst.brect().expand_side(Side::Right, 1_400));
        }

        // Block extra for decoder to prevent extra power straps from being placed.
        router.block(
            m2,
            decoder
                .layer_bbox(m2)
                .into_rect()
                .expand_side(Side::Right, 6_000),
        );

        // Block entirety of bounding box for bitcells, replica bitcells, and column peripherals.
        for inst in [&bitcells, &rbl, &cols] {
            router.block(
                m1,
                inst.brect()
                    .expand_dir(Dir::Vert, 6_000)
                    .expand_dir(Dir::Horiz, 140),
            );
            router.block(
                m2,
                inst.brect()
                    .expand_dir(Dir::Horiz, 6_000)
                    .expand_dir(Dir::Vert, 140),
            );
        }

        // Route precharges to bitcell array.
        for i in 0..self.params.cols() {
            for port_name in ["bl", "br"] {
                let src = cols.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                let dst = bitcells.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                ctx.draw_rect(m1, src.union(dst.bbox()).into_rect());
            }
        }

        // Route DFF input signals to pins on bounding box of SRAM on m1.
        for i in 0..dsn.num_dffs {
            let src = dffs.port(PortId::new("d", i))?.largest_rect(m0)?;
            let track_span =
                m1_tracks.index(m1_tracks.track_with_loc(TrackLocator::Nearest, src.center().x));
            let m1_rect = src
                .with_hspan(track_span)
                .with_vspan(src.vspan().add_point(router_bbox.bottom()));
            draw_rect(m1, m1_rect, &mut router, ctx);
            ctx.add_port(
                CellPort::builder()
                    .id(if i == dsn.num_dffs - 1 {
                        "we".into()
                    } else if i == dsn.num_dffs - 2 {
                        "ce".into()
                    } else {
                        PortId::new("addr", self.params.addr_width() - i - 1)
                    })
                    .add(m1, m1_rect)
                    .build(),
            )?;
            let via = draw_via(m0, src, m1, src, ctx)?;
            let expanded_rect = via.layer_bbox(m1).into_rect();
            let expanded_rect = expanded_rect.with_hspan(expanded_rect.hspan().union(track_span));
            draw_rect(m1, expanded_rect, &mut router, ctx);
        }

        // Route address gate to predecoders.
        let addr_gate_m1_track_idx =
            m1_tracks.track_with_loc(TrackLocator::StartsAfter, addr_gate.brect().right() + 700);
        // Maps predecoder port spans to the previous m2 track used to connect to the input bus.
        // Increments each time a track is used.
        let mut m2_prev_track = HashMap::new();
        for i in 0..self.params.row_bits() {
            for j in 0..2 {
                let idx = 2 * i + j;
                let y = addr_gate.port(PortId::new("y", idx))?.largest_rect(m0)?;
                let predecode_port = decoder
                    .port(format!("predecode_{}_{}", i, j))?
                    .largest_rect(m1)?;

                // Choose the track to use to connect to the predecoder input bus.
                // If bus has already been connected to, use next track.
                // Otherwise, use bottom most track contained by the bus's vertical span.
                let m2_track_final_idx = *m2_prev_track
                    .entry(predecode_port.vspan())
                    .and_modify(|v| *v += 1)
                    .or_insert(
                        m2_tracks
                            .track_with_loc(TrackLocator::StartsAfter, predecode_port.bottom()),
                    );

                // Jog the address gate output port to the nearest m2 track.
                let m2_track_idx = m2_tracks.track_with_loc(TrackLocator::Nearest, y.top());
                // Expand port to make space for via.
                let rect = y.expand_side(Side::Right, 340);
                ctx.draw_rect(m0, rect);
                let via_rect = rect.with_hspan(Span::with_stop_and_length(rect.right(), 340));
                let via = draw_via(m0, via_rect, m1, via_rect, ctx)?;
                // Expand m1 rect of via to overlap with m2 track.
                let via_m1 = via.layer_bbox(m1).into_rect();

                // Determine m1 track to jog the signal vertically. If the signal needs to jog
                // downwards, need the m1 track number to increase as we move upward through
                // the address gate outputs.
                let m1_track_idx = if m2_track_final_idx < m2_track_idx {
                    addr_gate_m1_track_idx + idx as i64
                } else {
                    addr_gate_m1_track_idx + 2 * self.params.row_bits() as i64 - 1 - idx as i64
                };
                draw_route(
                    m1,
                    via_m1,
                    m1,
                    predecode_port,
                    Dir::Horiz,
                    vec![m2_track_idx, m1_track_idx, m2_track_final_idx],
                    &mut router,
                    ctx,
                )?;
            }
        }

        let mut track_idx =
            m1_tracks.track_with_loc(TrackLocator::StartsAfter, pc_b_buffer.brect().right()) + 2;
        let mut buffers = vec![
            (&pc_b_buffer, "pc_b", m2, dsn.pc_b_routing_tracks),
            (
                &sense_en_buffer,
                "sense_en",
                m2,
                dsn.sense_en_routing_tracks,
            ),
        ];
        if sense_en_buffer
            .brect()
            .vspan()
            .intersects(&cols.port("sense_en")?.largest_rect(m2)?.vspan())
        {
            buffers.reverse();
        }

        buffers.push((
            &write_driver_en_buffer,
            "we",
            m1,
            dsn.write_driver_en_routing_tracks,
        ));
        // Route buffers to columns.
        for (buffer, signal, layer, num_tracks) in buffers {
            let track = (0..num_tracks)
                .map(|i| {
                    m1_tracks.index(if signal == "we" {
                        m1_tracks
                            .track_with_loc(TrackLocator::EndsBefore, cols.brect().left() - 5_400)
                            - i
                    } else {
                        track_idx + i
                    })
                })
                .reduce(|a, b| a.union(b))
                .unwrap();

            for buf_port in buffer
                .port("y")?
                .shapes(m1)
                .filter_map(|shape| shape.as_rect())
            {
                draw_rect(m1, buf_port.expand_side(Side::Right, 200), &mut router, ctx);
            }

            for col_port in cols
                .port(signal)?
                .shapes(layer)
                .filter_map(|shape| shape.as_rect())
            {
                let m2_rect =
                    Rect::from_spans(track.add_point(col_port.left() + 1_400), col_port.vspan());
                draw_via(m1, m2_rect.with_hspan(track), m2, m2_rect, ctx)?;
                draw_rect(m2, m2_rect, &mut router, ctx);
                if layer == m1 {
                    draw_via(m1, col_port, m2, m2_rect, ctx)?;
                }
            }

            let buffer_port_vspan = buffer
                .port("y")?
                .first_rect(m1, Side::Top)?
                .vspan()
                .union(buffer.port("y")?.first_rect(m1, Side::Bot)?.vspan());

            for i in 0..std::cmp::max((buffer_port_vspan.length() - 2_000) / 4_000 + 1, 1) {
                let span =
                    Span::with_start_and_length(buffer_port_vspan.start() + 4_000 * i, 2_000);
                let span = Span::new(
                    span.start(),
                    if span.stop() > buffer_port_vspan.stop() {
                        buffer_port_vspan.stop()
                    } else {
                        span.stop()
                    },
                );
                if span.length() > 140 {
                    draw_rect(
                        m1,
                        Rect::from_spans(
                            track.add_point(buffer.port("y")?.largest_rect(m1)?.right() + 200),
                            span,
                        ),
                        &mut router,
                        ctx,
                    );
                }
            }
            router.block(
                m1,
                Rect::from_spans(
                    track.add_point(buffer.port("y")?.largest_rect(m1)?.right() + 200),
                    buffer_port_vspan,
                ),
            );
            let col_port_vspan = cols
                .port(signal)?
                .first_rect(layer, Side::Top)?
                .vspan()
                .union(cols.port(signal)?.first_rect(layer, Side::Bot)?.vspan());
            let m1_rect = Rect::from_spans(track, buffer_port_vspan.union(col_port_vspan));
            draw_rect(m1, m1_rect, &mut router, ctx);

            track_idx += num_tracks;
        }

        // Route wordline driver to bitcell array
        for i in 0..self.params.rows() {
            let src = decoder.port(PortId::new("y", i))?.largest_rect(m1)?;
            let src = src.with_hspan(Span::with_stop_and_length(src.right() + 600, 1_200));
            let dst = bitcells.port(PortId::new("wl", i))?.largest_rect(m2)?;
            let via = draw_via(m1, src, m2, src, ctx)?;
            router.block(m1, via.bbox().into_rect());
            let m2_rect_a = via.layer_bbox(m2).into_rect();
            let m2_rect_a = m2_rect_a.with_vspan(m2_rect_a.vspan().union(dst.vspan()));
            let m2_rect_b = dst.with_hspan(m2_rect_a.hspan().union(dst.hspan()));
            draw_rect(m2, m2_rect_a, &mut router, ctx);
            draw_rect(m2, m2_rect_b, &mut router, ctx);
        }

        // Route column decoders to mux.
        for i in 0..self.params.mux_ratio() {
            for j in 0..2 {
                let (y_name, sel_name) = if j == 0 {
                    ("y_b", "sel_b")
                } else {
                    ("y", "sel")
                };
                let idx = (2 * i + j) as i64 * dsn.col_dec_routing_tracks;
                let track_span = (0..dsn.col_dec_routing_tracks)
                    .map(|i| m1_tracks.index(track_idx + i + idx))
                    .reduce(|a, b| a.union(b))
                    .unwrap();
                let mut vspans = Vec::new();
                for y in col_dec
                    .port(PortId::new(y_name, i))?
                    .shapes(m1)
                    .filter_map(|shape| shape.as_rect())
                {
                    let via_width = 1_400;
                    let y = y.expand_side(Side::Right, 220 + via_width);
                    draw_rect(m1, y, &mut router, ctx);
                    let m2_rect = y.with_hspan(
                        Span::with_stop_and_length(y.hspan().stop(), via_width).union(track_span),
                    );
                    draw_via(m1, y, m2, m2_rect, ctx)?;
                    vspans.push(m2_rect.vspan());
                    draw_rect(m2, m2_rect, &mut router, ctx);
                    draw_via(m1, m2_rect.with_hspan(track_span), m2, m2_rect, ctx)?;
                }

                for sel in cols
                    .port(PortId::new(sel_name, i))?
                    .shapes(m2)
                    .filter_map(|shape| shape.as_rect())
                {
                    let m2_rect = Rect::from_spans(track_span.union(sel.hspan()), sel.vspan());
                    vspans.push(m2_rect.vspan());
                    draw_rect(m2, m2_rect, &mut router, ctx);
                    draw_via(m1, m2_rect.with_hspan(track_span), m2, m2_rect, ctx)?;
                }
                let m1_rect = Rect::from_spans(
                    track_span,
                    vspans.into_iter().reduce(|a, b| a.union(b)).unwrap(),
                );
                draw_rect(m1, m1_rect, &mut router, ctx);
            }
        }

        // Route control logic inputs to m2 tracks above DFFs.
        let dff_m2_track_idx =
            m2_tracks.track_with_loc(TrackLocator::StartsAfter, dffs.brect().top());
        let control_m2_track_idx = dff_m2_track_idx + 2 * self.params.addr_width() as i64;
        let m2_clk_track_idx = control_m2_track_idx;
        let m2_rstb_track_idx = control_m2_track_idx + 1;
        let m2_ce_track_idx = control_m2_track_idx + 2;
        let m2_we_track_idx = control_m2_track_idx + 3;

        let m2_track_conn_idx = m2_tracks.track_with_loc(
            TrackLocator::EndsBefore,
            dffs.bbox().union(cols.bbox()).into_rect().bottom(),
        );
        let m2_track_clk_conn = m2_track_conn_idx;
        let m2_track_rstb_conn = m2_track_conn_idx - 1;

        // Route clk and rstb.
        // Connect clock ports from the left and clock ports on the right to separate
        // m1 tracks to prevent overlapping vias.
        let m1_clk_track_left_idx =
            m1_tracks.track_with_loc(TrackLocator::StartsAfter, dffs.brect().right());
        let m1_clk_track_right_idx =
            m1_tracks.track_with_loc(TrackLocator::EndsBefore, cols.brect().left() - 300);
        let m1_rstb_track_left_idx = m1_clk_track_left_idx + 1;
        let m1_rstb_track_right_idx = m1_clk_track_right_idx - 1;

        for (port, m1_track_left, m1_track_right, m2_track, m2_track_conn) in [
            (
                "clk",
                m1_clk_track_left_idx,
                m1_clk_track_right_idx,
                m2_clk_track_idx,
                m2_track_clk_conn,
            ),
            (
                "rstb",
                m1_rstb_track_left_idx,
                m1_rstb_track_right_idx,
                m2_rstb_track_idx,
                m2_track_rstb_conn,
            ),
        ] {
            let control_port = control.port(port)?.largest_rect(m1)?;

            // Draw pin on the edge of one of the m1 tracks.
            let m1_pin = Rect::from_spans(
                m1_tracks.index(m1_track_left),
                Span::with_start_and_length(router_bbox.bottom(), 320),
            );
            draw_rect(m1, m1_pin, &mut router, ctx);
            ctx.add_port(CellPort::with_shape(port, m1, m1_pin))?;

            // Connect control logic to pin.
            draw_route(
                m1,
                control_port,
                m1,
                m1_pin,
                Dir::Horiz,
                vec![m2_track],
                &mut router,
                ctx,
            )?;

            // Connect addr dffs and dffs from column peripherals to pin.
            for (inst, m1_track) in [(&dffs, m1_track_left), (&cols, m1_track_right)] {
                for port_rect in inst
                    .port(port)?
                    .shapes(m2)
                    .filter_map(|shape| shape.as_rect())
                {
                    draw_route(
                        m2,
                        port_rect,
                        m1,
                        m1_pin,
                        Dir::Vert,
                        vec![m1_track, m2_track_conn],
                        &mut router,
                        ctx,
                    )?;
                }
            }
        }

        // Route ce and we to DFFs.
        for (port, m2_track_idx, dff_idx) in [
            ("ce", m2_ce_track_idx, dsn.num_dffs - 2),
            ("we", m2_we_track_idx, dsn.num_dffs - 1),
        ] {
            let control_port = control.port(port)?.largest_rect(m1)?;
            let dff_port = dffs.port(PortId::new("q", dff_idx))?.largest_rect(m0)?;
            let via = draw_via(m0, dff_port, m1, dff_port, ctx)?;
            let via_m1 = via.layer_bbox(m1).into_rect();
            let track_idx = m1_tracks.track_with_loc(TrackLocator::Nearest, via_m1.center().x);
            draw_route(
                m1,
                via_m1,
                m1,
                control_port,
                Dir::Vert,
                vec![track_idx, m2_track_idx],
                &mut router,
                ctx,
            )?;
        }

        // Route replica cell array to replica precharge and replica_nmos
        for i in 0..2 {
            for port_name in ["bl", "br"] {
                let src = rbl.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                let dst = replica_pc
                    .port(PortId::new(format!("{}_in", port_name), i))?
                    .largest_rect(m1)?;
                ctx.draw_rect(m1, src.bbox().union(dst.bbox()).into_rect());
            }
            let src = replica_pc
                .port(PortId::new("bl_out", i))?
                .largest_rect(m1)?;
            let dst = replica_nmos.port(PortId::new("bl", i))?.largest_rect(m1)?;
            ctx.draw_rect(m1, src.bbox().union(dst.bbox()).into_rect());
        }

        // Route replica wordline.
        let control_rwl_rect = control.port("rwl")?.largest_rect(m2)?;
        let array_rwl_rect = rbl
            .port(PortId::new("wl", dsn.rbl_wl_index))?
            .largest_rect(m2)?;
        let m1_rwl_track_idx =
            m1_tracks.track_with_loc(TrackLocator::EndsBefore, control_rwl_rect.right());
        draw_route(
            m2,
            control_rwl_rect,
            m2,
            array_rwl_rect,
            Dir::Vert,
            vec![m1_rwl_track_idx],
            &mut router,
            ctx,
        )?;

        // Route replica bitline/precharge.
        let control_rbl_rect = control.port("rbl")?.largest_rect(m1)?;
        let control_pc_b_rect = control.port("pc_b")?.largest_rect(m1)?;
        let array_rbl_rect = replica_pc.port("rbl")?.largest_rect(m2)?;

        let m2_rbl_track_idx =
            m2_tracks.track_with_loc(TrackLocator::StartsAfter, control_rbl_rect.bottom());
        let m2_pc_b_track_idx = m2_rbl_track_idx + 1;
        let m2_wlen_track_idx = m2_rbl_track_idx + 2;

        let m1_rbl_track_idx =
            m1_tracks.track_with_loc(TrackLocator::StartsAfter, array_rbl_rect.right());
        let m1_pc_b_track_idx = m1_rbl_track_idx + 1;

        for (control_rect, array_rect, m1_track, m2_track) in std::iter::once((
            control_rbl_rect,
            array_rbl_rect,
            m1_rbl_track_idx,
            m2_rbl_track_idx,
        ))
        .chain(
            replica_pc
                .port("en_b")?
                .shapes(m2)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| {
                    (
                        control_pc_b_rect,
                        rect,
                        m1_pc_b_track_idx,
                        m2_pc_b_track_idx,
                    )
                }),
        ) {
            draw_route(
                m1,
                control_rect,
                m2,
                array_rect,
                Dir::Horiz,
                vec![m2_track, m1_track],
                &mut router,
                ctx,
            )?;
        }

        // Route wlen
        let control_wlen_rect = control.port("wlen")?.largest_rect(m1)?;
        let buffer_wlen_rect = wlen_buffer.port("predecode_0_0")?.largest_rect(m2)?;
        let m1_wlen_track_idx =
            m1_tracks.track_with_loc(TrackLocator::EndsBefore, buffer_wlen_rect.right());
        draw_route(
            m1,
            control_wlen_rect,
            m2,
            buffer_wlen_rect,
            Dir::Horiz,
            vec![m2_wlen_track_idx, m1_wlen_track_idx],
            &mut router,
            ctx,
        )?;

        let addr_gate_wlen_rect = addr_gate.port("wl_en")?.largest_rect(m1)?;
        let y = wlen_buffer.port("y")?.largest_rect(m1)?;
        let jog = OffsetJog::builder()
            .dir(subgeom::Dir::Vert)
            .sign(subgeom::Sign::Pos)
            .src(y)
            .dst(addr_gate_wlen_rect.left())
            .layer(m1)
            .space(200)
            .build()
            .unwrap();
        let m1_rect = jog
            .r2()
            .with_hspan(jog.r2().hspan().union(addr_gate_wlen_rect.hspan()));
        draw_rect(m1, m1_rect, &mut router, ctx);
        let m1_rect =
            addr_gate_wlen_rect.with_vspan(jog.r2().vspan().union(addr_gate_wlen_rect.vspan()));
        draw_rect(m1, m1_rect, &mut router, ctx);
        ctx.draw(jog)?;

        // Route pc_b to main array.
        let pc_b_rect = pc_b_buffer.port("predecode_0_0")?.largest_rect(m1)?;
        draw_route(
            m1,
            control_pc_b_rect,
            m1,
            pc_b_rect,
            Dir::Horiz,
            vec![m2_pc_b_track_idx],
            &mut router,
            ctx,
        )?;

        // Route sense_en and write_driver_en.
        let m1_write_driver_en_track_idx =
            m1_tracks.track_with_loc(TrackLocator::StartsAfter, control.brect().right() + 140);
        let m1_sense_en_track_idx = m1_write_driver_en_track_idx + 1;

        for (port, track, buf) in [
            ("saen", m1_sense_en_track_idx, &sense_en_buffer),
            (
                "wrdrven",
                m1_write_driver_en_track_idx,
                &write_driver_en_buffer,
            ),
        ] {
            let buffer_port = buf.port("predecode_0_0")?.largest_rect(m1)?;
            let control_port = control.port(port)?.largest_rect(m2)?;

            if buffer_port.vspan().contains(control_port.vspan()) {
                let m2_rect =
                    control_port.with_hspan(control_port.hspan().union(buffer_port.hspan()));
                draw_rect(m2, m2_rect, &mut router, ctx);
                draw_via(m1, buffer_port, m2, m2_rect, ctx)?;
            } else {
                let m2_track_idx = if buffer_port.vspan().start() > control_port.vspan().start() {
                    m2_tracks.track_with_loc(TrackLocator::StartsAfter, buffer_port.bottom())
                } else {
                    m2_tracks.track_with_loc(TrackLocator::EndsBefore, buffer_port.top())
                };
                draw_route(
                    m1,
                    buffer_port,
                    m2,
                    control_port,
                    Dir::Horiz,
                    vec![m2_track_idx, track],
                    &mut router,
                    ctx,
                )?;
            }
        }

        // Route column select bits.
        for i in 0..self.params.col_select_bits() {
            for j in 0..2 {
                let idx = 2 * i + j;
                let dff_idx = dsn.num_dffs - i - 3;
                let port_rect = col_dec
                    .port(format!("predecode_{i}_{j}"))?
                    .largest_rect(m1)?;
                let rect = if j == 0 {
                    dffs.port(PortId::new("q_n", dff_idx))?
                        .first_rect(m0, Side::Left)?
                } else {
                    dffs.port(PortId::new("q", dff_idx))?.largest_rect(m0)?
                };
                let (loc, side) = if j == 0 {
                    (TrackLocator::StartsAfter, Side::Left)
                } else {
                    (TrackLocator::EndsBefore, Side::Right)
                };
                let track_span = m1_tracks.index(
                    m1_tracks.track_with_loc(loc, rect.side(side) - 140 * side.sign().as_int()),
                );
                let m0_rect = rect.with_hspan(track_span);
                let via = draw_via(m0, m0_rect, m1, m0_rect, ctx)?;
                let dff_port = Rect::from_spans(track_span, via.layer_bbox(m1).into_rect().vspan());
                draw_rect(m1, dff_port, &mut router, ctx);

                let m2_track_a_idx = m2_tracks
                    .track_with_loc(TrackLocator::StartsAfter, port_rect.bottom())
                    + idx as i64;
                let m1_track_idx = m1_write_driver_en_track_idx + 2 + idx as i64;
                let m1_track = m1_tracks.index(m1_track_idx);
                let m2_track_b_idx = if m1_track.start() < dff_port.left() {
                    dff_m2_track_idx + 2 * self.params.addr_width() as i64 - 1 - idx as i64
                } else {
                    dff_m2_track_idx + 2 * self.params.row_bits() as i64 + idx as i64
                };

                draw_route(
                    m1,
                    port_rect,
                    m1,
                    dff_port,
                    Dir::Horiz,
                    vec![m2_track_a_idx, m1_track_idx, m2_track_b_idx],
                    &mut router,
                    ctx,
                )?;
            }
        }

        // Route row address bits to addr gate.
        for i in 0..self.params.row_bits() {
            for j in 0..2 {
                let idx = 2 * i + j;
                let dff_idx = dsn.num_dffs - i - 3 - self.params.col_select_bits();
                let port_rect = addr_gate.port(PortId::new("in", idx))?.largest_rect(m0)?;
                let rect = if j == 0 {
                    dffs.port(PortId::new("q", dff_idx))?.largest_rect(m0)?
                } else {
                    dffs.port(PortId::new("q_n", dff_idx))?
                        .first_rect(m0, Side::Left)?
                };
                let (loc, side) = if j == 0 {
                    (TrackLocator::EndsBefore, Side::Right)
                } else {
                    (TrackLocator::StartsAfter, Side::Left)
                };
                let track_span = m1_tracks.index(
                    m1_tracks.track_with_loc(loc, rect.side(side) - 140 * side.sign().as_int()),
                );
                let m0_rect = rect.with_hspan(track_span);
                let via = draw_via(m0, m0_rect, m1, m0_rect, ctx)?;
                let dff_port = Rect::from_spans(track_span, via.layer_bbox(m1).into_rect().vspan());
                draw_rect(m1, dff_port, &mut router, ctx);

                let m2_track_idx =
                    dff_m2_track_idx + 2 * self.params.row_bits() as i64 - 1 - idx as i64;
                let m1_track_idx = m1_tracks.track_with_loc(
                    TrackLocator::EndsBefore,
                    addr_gate
                        .brect()
                        .expand_side(Side::Left, 680)
                        .bbox()
                        .union(rbl.brect().expand_side(Side::Left, 6_000).bbox())
                        .union(wlen_buffer.brect().expand_side(Side::Left, 2_000).bbox())
                        .into_rect()
                        .left()
                        - 140,
                ) - idx as i64;
                let m1_track = m1_tracks.index(m1_track_idx);
                let m2_track = m2_tracks.index(m2_track_idx);

                let m2_rect = Rect::from_spans(
                    m1_track.add_point(port_rect.left() - 600),
                    Span::from_center_span_gridded(
                        port_rect.center().y,
                        320,
                        ctx.pdk().layout_grid(),
                    ),
                );
                let m0_rect = port_rect.with_hspan(
                    port_rect
                        .hspan()
                        .union(Span::with_stop_and_length(m2_rect.right(), 320)),
                );
                ctx.draw_rect(m0, m0_rect);
                draw_rect(m2, m2_rect, &mut router, ctx);
                let via = draw_via(m0, m0_rect, m1, m2_rect, ctx)?;
                router.block(m1, via.layer_bbox(m1).brect());
                let via = draw_via(m1, m0_rect, m2, m2_rect, ctx)?;
                router.block(m1, via.layer_bbox(m1).brect());
                draw_via(m1, m2_rect.with_hspan(m1_track), m2, m2_rect, ctx)?;

                draw_rect(
                    m1,
                    Rect::from_spans(m1_track, m2_rect.vspan().union(m2_track)),
                    &mut router,
                    ctx,
                );
                draw_rect(
                    m1,
                    Rect::from_spans(m1_track.union(track_span), m2_track),
                    &mut router,
                    ctx,
                );
                draw_rect(
                    m1,
                    Rect::from_spans(track_span, m2_track.union(dff_port.vspan())),
                    &mut router,
                    ctx,
                );
            }
        }

        let mut straps = RoutedStraps::new();
        straps.set_strap_layers([m1, m2]);

        // Helper function for connecting bitcell ports to power straps.
        let mut connect_bitcells_to_straps = |inst: &Instance,
                                              port_ids: Vec<(PortId, SingleSupplyNet)>|
         -> Result<()> {
            let target_brect = inst
                .brect()
                .expand_dir(Dir::Horiz, 5_720)
                .expand_dir(Dir::Vert, 3_500);
            for (dir, layer, extension) in [(Dir::Vert, m1, 3_300), (Dir::Horiz, m2, 5_520)] {
                let mut to_merge = HashMap::new();
                for (port_id, net) in port_ids.iter() {
                    for port in inst.port(port_id.clone())?.shapes(layer) {
                        let bitcell_center = inst.bbox().center().coord(dir);
                        if let Shape::Rect(rect) = port {
                            let sign = if rect.center().coord(dir) < bitcell_center {
                                Sign::Neg
                            } else {
                                Sign::Pos
                            };
                            let rect = rect.with_span(
                                rect.span(dir).add_point(target_brect.span(dir).point(sign)),
                                dir,
                            );
                            ctx.draw_rect(layer, rect);
                            to_merge
                                .entry((sign, *net))
                                .or_insert(Vec::new())
                                .push(rect.span(!dir));
                        }
                    }
                }
                for ((sign, net), spans) in to_merge.into_iter() {
                    let merged_spans = Span::merge_adjacent(spans, |a, b| a.min_distance(b) < 400);

                    for span in merged_spans {
                        let curr = Rect::span_builder()
                            .with(
                                dir,
                                Span::with_point_and_length(
                                    sign,
                                    target_brect.span(dir).point(sign),
                                    extension,
                                ),
                            )
                            .with(!dir, span)
                            .build();
                        ctx.draw_rect(layer, curr);
                        straps.add_target(layer, Target::new(net, curr));
                    }
                }
            }
            Ok(())
        };

        // Connect bitcell array to power straps.
        let mut port_ids: Vec<(PortId, SingleSupplyNet)> = ["vpwr", "vgnd", "vpb", "vnb"]
            .into_iter()
            .map(|x| {
                (
                    x.into(),
                    match x {
                        "vpwr" | "vpb" => SingleSupplyNet::Vdd,
                        "vgnd" | "vnb" => SingleSupplyNet::Vss,
                        _ => unreachable!(),
                    },
                )
            })
            .collect();
        for i in 0..2 {
            port_ids.push((PortId::new("wl_dummy", i), SingleSupplyNet::Vss));
            for port_name in ["bl_dummy", "br_dummy"] {
                port_ids.push((PortId::new(port_name, i), SingleSupplyNet::Vdd));
            }
        }

        connect_bitcells_to_straps(&bitcells, port_ids)?;

        // Connect replica bitcell array to power straps.
        let mut port_ids: Vec<(PortId, SingleSupplyNet)> = ["vpwr", "vgnd", "vpb", "vnb"]
            .into_iter()
            .map(|x| {
                (
                    x.into(),
                    match x {
                        "vpwr" | "vpb" => SingleSupplyNet::Vdd,
                        "vgnd" | "vnb" => SingleSupplyNet::Vss,
                        _ => unreachable!(),
                    },
                )
            })
            .collect();
        for i in 0..dsn.rbl.rows {
            if i != dsn.rbl_wl_index {
                port_ids.push((PortId::new("wl", i), SingleSupplyNet::Vss));
            }
        }
        connect_bitcells_to_straps(&rbl, port_ids)?;

        // Connect column circuitry to power straps.
        for layer in [m1, m2] {
            for port_name in ["vdd", "vss"] {
                for port in cols
                    .port(port_name)?
                    .shapes(layer)
                    .filter_map(|shape| shape.as_rect())
                    .filter(|rect| rect.height() < 5000)
                {
                    let new_span = cols.brect().hspan().expand_all(5_000);
                    if layer == m2 {
                        ctx.merge_port(CellPort::with_shape(
                            port_name,
                            m2,
                            Rect::from_spans(new_span, port.vspan()),
                        ));
                    }
                    for sign in [Sign::Neg, Sign::Pos] {
                        let rect = port.with_hspan(Span::new(
                            new_span.point(sign),
                            port.hspan().point(sign) - sign.as_int() * 800,
                        ));
                        if layer == m1 {
                            draw_via(m1, port, m2, rect, ctx)?;
                            ctx.merge_port(CellPort::with_shape(port_name, m2, rect));
                        }
                        draw_rect(m2, rect, &mut router, ctx);
                        straps.add_target(
                            m2,
                            Target::new(
                                match port_name {
                                    "vdd" => SingleSupplyNet::Vdd,
                                    "vss" => SingleSupplyNet::Vss,
                                    _ => unreachable!(),
                                },
                                rect,
                            ),
                        );
                    }
                }
            }
        }

        // Connect m1 power straps to grid.
        for inst in [
            &decoder,
            &addr_gate,
            &col_dec,
            &dffs,
            &control,
            &pc_b_buffer,
            &sense_en_buffer,
            &write_driver_en_buffer,
        ] {
            for port_name in ["vdd", "vss"] {
                for port in inst.port(port_name)?.shapes(m1) {
                    if let Shape::Rect(rect) = port {
                        straps.add_target(
                            m1,
                            Target::new(
                                match port_name {
                                    "vdd" => SingleSupplyNet::Vdd,
                                    "vss" => SingleSupplyNet::Vss,
                                    _ => unreachable!(),
                                },
                                rect,
                            ),
                        );
                    }
                }
            }
        }

        // Connect decoder m2 straps to power straps.
        for inst in [
            &decoder,
            &addr_gate,
            &col_dec,
            &pc_b_buffer,
            &sense_en_buffer,
            &write_driver_en_buffer,
        ] {
            for port_name in ["vdd", "vss"] {
                for port in inst
                    .port(port_name)?
                    .shapes(m2)
                    .filter_map(|shape| shape.as_rect())
                {
                    ctx.merge_port(CellPort::with_shape(port_name, m2, port));
                    let new_span = inst.brect().vspan().expand_all(2_000);
                    for sign in [Sign::Neg, Sign::Pos] {
                        let rect = port.with_vspan(Span::new(
                            new_span.point(sign),
                            port.vspan().point(sign) - sign.as_int() * 800,
                        ));
                        draw_rect(m1, rect, &mut router, ctx);
                        straps.add_target(
                            m1,
                            Target::new(
                                match port_name {
                                    "vdd" => SingleSupplyNet::Vdd,
                                    "vss" => SingleSupplyNet::Vss,
                                    _ => unreachable!(),
                                },
                                rect,
                            ),
                        );
                    }
                }
            }
        }

        // Connect m2 power straps to grid.
        for (inst, port_names, expand) in [
            (&wlen_buffer, vec!["vdd", "vss"], 2_000),
            (&replica_pc, vec!["vdd"], 5_520),
            (&replica_nmos, vec!["vss"], 5_520),
        ] {
            for port_name in port_names {
                for port in inst.port(port_name)?.shapes(m2) {
                    if let Shape::Rect(rect) = port {
                        let rect = rect.expand_dir(Dir::Horiz, expand);
                        ctx.merge_port(CellPort::with_shape(port_name, m2, rect));
                        draw_rect(m2, rect, &mut router, ctx);
                        straps.add_target(
                            m2,
                            Target::new(
                                match port_name {
                                    "vdd" => SingleSupplyNet::Vdd,
                                    "vss" => SingleSupplyNet::Vss,
                                    _ => unreachable!(),
                                },
                                rect,
                            ),
                        );
                    }
                }
            }
        }

        let needs_diodes = cols.brect().bottom() - router_bbox.bottom() > 50_000;
        // Route column peripheral outputs to pins on bounding box of SRAM
        let groups = self.params.data_width();
        for (j, (port, width)) in [
            ("dout", groups),
            ("din", groups),
            ("wmask", self.params.wmask_width()),
        ]
        .into_iter()
        .enumerate()
        {
            for i in 0..width {
                let port_id = PortId::new(port, i);
                let rect = cols.port(port_id.clone())?.largest_rect(m1)?;
                let rect = rect.with_vspan(rect.vspan().add_point(router_bbox.bottom()));
                draw_rect(m1, rect, &mut router, ctx);
                ctx.add_port(CellPort::builder().id(port_id).add(m1, rect).build())?;

                if port != "dout" && needs_diodes {
                    let mut diode = ctx
                        .instantiate::<TappedDiode>(&NoParams)?
                        .with_orientation(Named::R90);
                    diode.align(
                        AlignMode::Left,
                        rect,
                        match port {
                            "din" => -1_200,
                            "wmask" => -1_400,
                            _ => unreachable!(),
                        },
                    );
                    diode.align(AlignMode::Beneath, &cols, 6_000 * (j + 1) as i64);
                    let diode_port = diode.port("diode")?.largest_rect(m0)?;
                    let diode_via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m0, m1)
                            .geometry(diode_port, rect)
                            .build(),
                    )?;
                    ctx.draw(diode_via)?;
                    for shape in diode.shapes_on(m1) {
                        let rect = shape.brect();
                        router.block(m1, rect);
                    }
                    for (port, net) in [
                        ("vpwr", SingleSupplyNet::Vdd),
                        ("vgnd", SingleSupplyNet::Vss),
                    ] {
                        straps.add_target(m1, Target::new(net, diode.port(port)?.bbox(m1)));
                    }
                    ctx.draw(diode)?;
                }
            }
        }
        ctx.set_metadata(if needs_diodes {
            NeedsDiodes::Yes
        } else {
            NeedsDiodes::No
        });

        let straps = straps.fill(&router, ctx)?;
        ctx.set_metadata(straps);

        ctx.draw(router)?;
        Ok(())
    }
}
