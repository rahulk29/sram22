use std::collections::{HashMap, VecDeque};

use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Named;
use subgeom::transform::Translate;
use subgeom::{Corner, Dir, Point, Rect, Shape, Side, Sign, Span};
use substrate::component::{Component, NoParams};
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Instance, Port, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::{LayerBoundBox, LayerKey};
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::tile::LayerBbox;
use substrate::layout::routing::auto::grid::{
    ExpandToGridStrategy, JogToGrid, OffGridBusTranslation, OffGridBusTranslationStrategy,
};
use substrate::layout::routing::auto::straps::{RoutedStraps, Target};
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::{ElbowJog, OffsetJog, SJog};
use substrate::layout::routing::tracks::{TrackLocator, UniformTracks};
use substrate::layout::straps::SingleSupplyNet;
use substrate::layout::Draw;
use substrate::pdk::stdcell::StdCell;

use crate::blocks::bitcell_array::replica::{ReplicaCellArray, ReplicaCellArrayParams};
use crate::blocks::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::blocks::columns::ColPeripherals;
use crate::blocks::control::{ControlLogicReplicaV2, DffArray};
use crate::blocks::decoder::{
    Decoder, DecoderParams, DecoderStage, DecoderStageParams, DecoderTree, INV_PARAMS, NAND2_PARAMS,
};
use crate::blocks::gate::{AndParams, GateParams};
use crate::blocks::precharge::layout::{ReplicaPrecharge, ReplicaPrechargeParams};

use super::schematic::fanout_buffer_stage;
use super::{SramInner, SramPhysicalDesignScript};

pub struct TappedDiode;

impl Component for TappedDiode {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_diode")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;

        let tap = lib.try_cell_named("sky130_fd_sc_hd__tap_2")?;
        let tap = ctx.instantiate::<StdCell>(&tap.id())?;
        let tap = LayerBbox::new(tap, outline);
        let diode = lib.try_cell_named("sky130_fd_sc_hd__diode_2")?;
        let diode = ctx.instantiate::<StdCell>(&diode.id())?;
        let diode = LayerBbox::new(diode, outline);

        let mut row = ArrayTiler::builder();
        row.mode(AlignMode::ToTheRight).alt_mode(AlignMode::Top);
        row.push(tap.clone());
        row.push(diode);
        row.push(tap);
        let mut row = row.build();
        row.expose_ports(
            |port: CellPort, i| {
                if i == 1 || port.name() == "vpwr" || port.name() == "vgnd" {
                    Some(port)
                } else {
                    None
                }
            },
            PortConflictStrategy::Merge,
        )?;
        let group = row.generate()?;
        ctx.add_ports(group.ports())?;
        ctx.draw(group)?;

        Ok(())
    }
}

/// Returns the layer used for routing in the provided direction.
///
/// The SRAM top level only uses m1 and m2 for vertical and horizontal routing, respectively.
fn get_layer(dir: Dir, ctx: &LayoutCtx) -> Result<LayerKey> {
    ctx.layers().get(Selector::Metal(match dir {
        Dir::Horiz => 2,
        Dir::Vert => 1,
    }))
}

/// Draws a route from `start` to `end` using the provided `tracks`.
fn draw_route(
    start: Rect,
    end: Rect,
    dir: Dir,
    tracks: Vec<i64>,
    router: &mut GreedyRouter,
    ctx: &mut LayoutCtx,
) -> Result<()> {
    let mut spans = vec![start.span(dir)];
    spans.extend(tracks.iter().enumerate().map(|(i, track)| {
        router
            .track_info(get_layer(if i % 2 == 0 { dir } else { !dir }, ctx).unwrap())
            .tracks()
            .index(*track)
    }));
    spans.push(end.span(if tracks.len() % 2 == 0 { !dir } else { dir }));
    let mut curr_dir = dir;
    let mut prev_rect = Some(start);
    for spans in spans.windows(3) {
        let (prev_track_span, curr_track_span, next_track_span) = (spans[0], spans[1], spans[2]);
        if prev_track_span == next_track_span {
            prev_rect = None;
        } else {
            let curr_layer = get_layer(curr_dir, ctx)?;
            let next_layer = get_layer(!curr_dir, ctx)?;
            let rect = Rect::span_builder()
                .with(curr_dir, prev_track_span.union(next_track_span))
                .with(!curr_dir, curr_track_span)
                .build();
            ctx.draw_rect(curr_layer, rect);
            router.block(curr_layer, rect);
            if let Some(prev_rect) = prev_rect {
                let (bot_rect, top_rect, bot_layer, top_layer) = match curr_dir {
                    Dir::Horiz => (prev_rect, rect, next_layer, curr_layer),
                    Dir::Vert => (rect, prev_rect, curr_layer, next_layer),
                };
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(bot_layer, top_layer)
                        .geometry(bot_rect, top_rect)
                        .build(),
                )?;
                ctx.draw(via)?;
            }
            prev_rect = Some(rect);
        }
        curr_dir = !curr_dir;
    }
    if let Some(prev_rect) = prev_rect {
        let curr_layer = get_layer(curr_dir, ctx)?;
        let next_layer = get_layer(!curr_dir, ctx)?;
        let (bot_rect, top_rect, bot_layer, top_layer) = match curr_dir {
            Dir::Horiz => (prev_rect, end, next_layer, curr_layer),
            Dir::Vert => (end, prev_rect, curr_layer, next_layer),
        };
        let via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(bot_layer, top_layer)
                .geometry(bot_rect, top_rect)
                .build(),
        )?;
        ctx.draw(via)?;
    }
    Ok(())
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
        let mut control = ctx.instantiate::<ControlLogicReplicaV2>(&NoParams)?;
        let mut pc_b_buffer = ctx
            .instantiate::<DecoderStage>(&dsn.pc_b_buffer)?
            .with_orientation(Named::R90Cw);
        let mut wl_en_buffer = ctx
            .instantiate::<DecoderStage>(&dsn.wlen_buffer)?
            .with_orientation(Named::R90Cw);
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

        // Align row decoders to left of bitcell array.
        decoder.align_to_the_left_of(bitcells.bbox(), 6_000);
        decoder.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());

        // Align address gate to the left of the row decoder.
        addr_gate.align_to_the_left_of(
            decoder.bbox(),
            700 + 1_400 * self.params.addr_width() as i64,
        );
        addr_gate.align_bottom(decoder.bbox());

        // Align column peripherals under bitcell array.
        cols.align_beneath(bitcells.bbox(), 4_000);
        cols.align_centers_horizontally_gridded(bitcells.bbox(), ctx.pdk().layout_grid());

        // Align pc_b buffer with pc_b port of column peripherals.
        pc_b_buffer.align_bottom(cols.port("pc_b")?.largest_rect(m2)?);
        pc_b_buffer.translate(Point::new(0, 1_000));
        pc_b_buffer.align_to_the_left_of(cols.bbox(), 6_000);

        // Align column decoder to topmost column mux select port.
        col_dec.align_top(
            cols.port(PortId::new("sel", self.params.mux_ratio() - 1))?
                .largest_rect(m2)?,
        );
        col_dec.align_to_the_left_of(
            cols.bbox(),
            std::cmp::max(6_000, 1_400 + 700 * self.params.mux_ratio() as i64),
        );
        col_dec.translate(Point::new(0, -1_000));

        // Align sense_en buffer as close to column peripheral sense_en port as possible.
        sense_en_buffer.align_beneath(
            col_dec.brect().expand(4_000).with_vspan(
                col_dec
                    .brect()
                    .vspan()
                    .add_point(cols.port("sense_en")?.largest_rect(m2)?.top()),
            ),
            0,
        );
        sense_en_buffer.align_to_the_left_of(cols.bbox(), 6_000);

        // Align write driver underneath sense_en buffer.
        write_driver_en_buffer.align_beneath(sense_en_buffer.bbox(), 4_000);
        write_driver_en_buffer.align_to_the_left_of(cols.bbox(), 6_000);

        // Align control logic to the left of all of the buffers and column decoder that border
        // column peripherals.
        let buffer_bbox = col_dec
            .bbox()
            .union(sense_en_buffer.bbox())
            .union(pc_b_buffer.bbox())
            .union(write_driver_en_buffer.bbox())
            .into_rect();
        control.set_orientation(Named::R90);
        control.align_beneath(decoder.bbox(), 4_000);
        control.align_to_the_left_of(
            buffer_bbox,
            2_100 + 1_400 * self.params.col_select_bits() as i64,
        );

        // Align replica bitcell array to left of control logic, with replica precharge
        // aligned to top of control logic.
        rbl.align_to_the_left_of(control.bbox(), 7_000);
        replica_pc.align_beneath(decoder.bbox(), 4_000);
        replica_pc.align_centers_horizontally_gridded(rbl.bbox(), ctx.pdk().layout_grid());
        rbl.align_beneath(replica_pc.bbox(), 4_000);

        // Align DFFs to the left of column peripherals and underneath all other objects.
        dffs.align_to_the_left_of(cols.bbox(), 8_000);
        dffs.align_beneath(
            control
                .bbox()
                .union(rbl.bbox())
                .union(write_driver_en_buffer.bbox()),
            3_500 + 1_400 * self.params.row_bits() as i64,
        );

        // Draw instances.
        ctx.draw_ref(&bitcells)?;
        ctx.draw_ref(&cols)?;
        ctx.draw_ref(&decoder)?;
        ctx.draw_ref(&addr_gate)?;
        ctx.draw_ref(&pc_b_buffer)?;
        ctx.draw_ref(&col_dec)?;
        ctx.draw_ref(&sense_en_buffer)?;
        ctx.draw_ref(&write_driver_en_buffer)?;
        ctx.draw_ref(&control)?;
        ctx.draw_ref(&dffs)?;
        ctx.draw_ref(&rbl)?;
        ctx.draw_ref(&replica_pc)?;

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

        // Block appropriate areas in router for each instance.
        for inst in [
            &addr_gate,
            &decoder,
            &col_dec,
            &pc_b_buffer,
            &sense_en_buffer,
            &write_driver_en_buffer,
            &dffs,
            &control,
            &replica_pc,
        ] {
            for layer in [m1, m2] {
                for shape in inst.shapes_on(layer) {
                    let rect = shape.brect();
                    router.block(layer, rect);
                }
            }
        }

        // Block entirety of bounding box for bitcells, replica bitcells, and column peripherals.
        for inst in [&bitcells, &rbl, &cols] {
            router.block(m1, inst.brect().expand_dir(Dir::Vert, 6_000));
            router.block(m2, inst.brect().expand_dir(Dir::Horiz, 6_000));
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
            let tracks = router.track_info(m1).tracks();
            let track_span =
                tracks.index(tracks.track_with_loc(TrackLocator::Nearest, src.center().x));
            let m1_rect = src
                .with_hspan(track_span)
                .with_vspan(src.vspan().add_point(router_bbox.bottom()));
            ctx.draw_rect(m1, m1_rect);
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
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(src, src)
                    .build(),
            )?;
            let expanded_rect = via.layer_bbox(m1).into_rect();
            let expanded_rect = expanded_rect.with_hspan(expanded_rect.hspan().union(track_span));
            ctx.draw(via)?;
            ctx.draw_rect(m1, expanded_rect);
            router.block(m1, expanded_rect);
            router.block(m1, m1_rect);
        }

        // Route address gate to predecoders.
        let addr_gate_m1_track_idx = router
            .track_info(m1)
            .tracks()
            .track_with_loc(TrackLocator::StartsAfter, addr_gate.brect().right() + 700);
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
                        router
                            .track_info(m2)
                            .tracks()
                            .track_with_loc(TrackLocator::StartsAfter, predecode_port.bottom()),
                    );

                // Jog the address gate output port to the nearest m2 track.
                let m2_tracks = router.track_info(m2).tracks();
                let m2_track_idx = m2_tracks.track_with_loc(TrackLocator::Nearest, y.top());
                let m2_track = m2_tracks.index(m2_track_idx);
                // Expand port to make space for via.
                let rect = y.expand_side(Side::Right, 340);
                ctx.draw_rect(m0, rect);
                let via_rect = rect.with_hspan(Span::with_stop_and_length(rect.right(), 340));
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(via_rect, via_rect)
                        .build(),
                )?;
                ctx.draw_ref(&via)?;
                // Expand m1 rect of via to overlap with m2 track.
                let m1_rect = via.layer_bbox(m1).into_rect();
                let m1_rect = m1_rect.with_vspan(m1_rect.vspan().union(m2_track));
                ctx.draw_rect(m1, m1_rect);
                router.block(m1, m1_rect);

                // Determine m1 track to jog the signal vertically. If the signal needs to jog
                // downwards, need the m1 track number to increase as we move upward through
                // the address gate outputs.
                let m1_track_idx = if m2_track_final_idx < m2_track_idx {
                    addr_gate_m1_track_idx + idx as i64
                } else {
                    addr_gate_m1_track_idx + 2 * self.params.row_bits() as i64 - 1 - idx as i64
                };
                draw_route(
                    m1_rect,
                    predecode_port,
                    Dir::Horiz,
                    vec![m2_track_idx, m1_track_idx, m2_track_final_idx],
                    &mut router,
                    ctx,
                )?;
            }
        }

        // Route buffers to columns.
        for (buffer, signal, layer) in [
            (&pc_b_buffer, "pc_b", m2),
            (&sense_en_buffer, "sense_en", m2),
            (&write_driver_en_buffer, "we", m1),
        ] {
            let y = buffer.port("y")?.largest_rect(m0)?;
            let col_port = cols.port(signal)?.largest_rect(layer)?;
            let tracks = router.track_info(m1).tracks();
            let track_idx =
                tracks.track_with_loc(TrackLocator::EndsBefore, cols.brect().left() - 300);
            let track = tracks.index(track_idx);

            let rect = y.with_hspan(y.hspan().union(track));
            ctx.draw_rect(m0, rect);
            let m1_rect = Rect::from_spans(track, y.vspan().union(col_port.vspan()));
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(rect, m1_rect)
                    .build(),
            )?;
            ctx.draw_ref(&via)?;
            let m2_rect = Rect::from_spans(
                m1_rect.hspan().add_point(col_port.left() + 320),
                col_port.vspan(),
            );
            ctx.draw_rect(m1, m1_rect);
            ctx.draw_rect(m2, m2_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw_ref(&via)?;
            router.block(m1, m1_rect);
            router.block(m2, m2_rect);
            if layer == m1 {
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(m2_rect, col_port)
                        .build(),
                )?;
                ctx.draw_ref(&via)?;
            }
        }

        // Route wordline driver to bitcell array
        for i in 0..self.params.rows() {
            let src = decoder.port(PortId::new("y", i))?.largest_rect(m0)?;
            let src = src.with_hspan(Span::with_stop_and_length(src.right(), 170));
            let dst = bitcells.port(PortId::new("wl", i))?.largest_rect(m2)?;
            let jog = SJog::builder()
                .src(src)
                .dst(dst)
                .dir(Dir::Horiz)
                .layer(m2)
                .width(170)
                .l1(0)
                .grid(ctx.pdk().layout_grid())
                .build()
                .unwrap();
            let jog_group = jog.draw()?;
            router.block(m2, jog_group.bbox().into_rect());
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(src, src)
                    .build(),
            )?;
            router.block(m1, via.bbox().into_rect());
            ctx.draw(via)?;
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(src, src)
                    .build(),
            )?;
            router.block(m1, via.bbox().into_rect());
            ctx.draw(via)?;
            ctx.draw(jog_group)?;
        }

        // Route column decoders to mux.
        let tracks = router.track_info(m1).tracks();
        let track_idx = tracks.track_with_loc(TrackLocator::StartsAfter, col_dec.brect().right());
        for i in 0..self.params.mux_ratio() {
            for j in 0..2 {
                let (y_name, sel_name) = if j == 0 {
                    ("y", "sel")
                } else {
                    ("y_b", "sel_b")
                };
                let y = col_dec.port(PortId::new(y_name, i))?.largest_rect(m0)?;
                let sel = cols.port(PortId::new(sel_name, i))?.largest_rect(m2)?;
                let tracks = router.track_info(m1).tracks();
                let track_span = tracks.index(
                    track_idx
                        + i as i64
                        + if (j == 0 && y.top() < sel.top())
                            || (j == 1 && y.bottom() > sel.bottom())
                        {
                            1
                        } else {
                            0
                        },
                );

                let rect = if j == 0 {
                    y.with_hspan(y.hspan().union(track_span))
                } else {
                    let jog = OffsetJog::builder()
                        .dir(subgeom::Dir::Horiz)
                        .sign(subgeom::Sign::Pos)
                        .src(y)
                        .dst(y.bottom() - 340)
                        .layer(m0)
                        .space(170)
                        .build()
                        .unwrap();
                    let rect = Rect::from_spans(
                        jog.r2().hspan().union(track_span),
                        Span::with_start_and_length(jog.r2().bottom(), 170),
                    );
                    ctx.draw(jog)?;
                    rect
                };
                let track_rect = Rect::from_spans(track_span, rect.vspan().union(sel.vspan()));
                ctx.draw_rect(m0, rect);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(rect, track_rect)
                        .build(),
                )?;
                ctx.draw_ref(&via)?;
                let m2_rect = Rect::from_spans(track_rect.hspan().union(sel.hspan()), sel.vspan());
                ctx.draw_rect(m1, track_rect);
                ctx.draw_rect(m2, m2_rect);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(track_rect, m2_rect)
                        .build(),
                )?;
                ctx.draw_ref(&via)?;
                router.block(m1, track_rect);
                router.block(m2, m2_rect);
            }
        }

        // Route control logic inputs to m2 tracks right above DFFs.
        let m2_tracks = router.track_info(m2).tracks();
        let dff_m2_track_idx =
            m2_tracks.track_with_loc(TrackLocator::StartsAfter, dffs.brect().top());
        let control_m2_track_idx = dff_m2_track_idx + 2 * self.params.row_bits() as i64;
        let m2_clk_track = m2_tracks.index(control_m2_track_idx);
        let m2_reset_b_track = m2_tracks.index(control_m2_track_idx + 1);
        let m2_ce_track = m2_tracks.index(control_m2_track_idx + 2);
        let m2_we_track = m2_tracks.index(control_m2_track_idx + 3);

        // Route clk and reset_b
        let m1_tracks = router.track_info(m1).tracks();
        let m1_track_idx =
            m1_tracks.track_with_loc(TrackLocator::StartsAfter, dffs.brect().right());
        let m1_clk_track = m1_tracks.index(m1_track_idx);
        let m1_reset_b_track = m1_tracks.index(m1_track_idx + 1);

        // Route addr dff clk/reset_b
        for (port, m1_track, m2_track) in [
            ("clk", m1_clk_track, m2_clk_track),
            ("reset_b", m1_reset_b_track, m2_reset_b_track),
        ] {
            let control_port = control.port(port)?.largest_rect(m1)?;
            let m1_rect = control_port.with_vspan(control_port.vspan().union(m2_track));
            let m2_rect = Rect::from_spans(control_port.hspan().union(m1_track), m2_track);
            ctx.draw_rect(m1, m1_rect);
            ctx.draw_rect(m2, m2_rect);
            router.block(m1, m1_rect);
            router.block(m2, m2_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;

            let m1_pin = Rect::from_spans(m1_track, m2_track.add_point(router_bbox.bottom()));
            ctx.add_port(CellPort::with_shape(port, m1, m1_pin))?;
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_pin, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;

            for inst in [&dffs, &cols] {
                for port_rect in inst
                    .port(port)?
                    .shapes(m2)
                    .filter_map(|shape| shape.as_rect())
                {
                    let m2_rect = port_rect.with_hspan(port_rect.hspan().union(m1_track));
                    let m1_rect = Rect::from_spans(
                        m1_track,
                        m2_rect
                            .vspan()
                            .union(m2_track)
                            .add_point(router_bbox.bottom()),
                    );
                    ctx.draw_rect(m1, m1_rect);
                    ctx.draw_rect(m2, m2_rect);
                    router.block(m1, m1_rect);
                    router.block(m2, m2_rect);
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m1, m2)
                            .geometry(m1_rect, m2_rect)
                            .build(),
                    )?;
                    ctx.draw(via)?;
                }
            }
        }

        // Route ce and we to DFFs.
        for (port, m2_track, dff_idx) in [
            ("ce", m2_ce_track, dsn.num_dffs - 2),
            ("we", m2_we_track, dsn.num_dffs - 1),
        ] {
            let control_port = control.port(port)?.largest_rect(m1)?;
            let dff_port = dffs.port(PortId::new("q", dff_idx))?.largest_rect(m0)?;
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(dff_port, dff_port)
                    .build(),
            )?;
            let expanded_rect = router.expand_to_layer_grid(
                via.layer_bbox(m1).into_rect(),
                m1,
                ExpandToGridStrategy::Side(Side::Top),
            );
            ctx.draw(via)?;
            ctx.draw_rect(m1, expanded_rect);
            let tracks = router.track_info(m1).tracks();
            let track_span = tracks
                .index(tracks.track_with_loc(TrackLocator::Nearest, expanded_rect.center().x));
            let m1_rect = Rect::from_spans(track_span, expanded_rect.vspan().union(m2_track));
            ctx.draw_rect(m1, m1_rect);
            router.block(m1, expanded_rect);
            router.block(m1, m1_rect);
            let m2_rect = Rect::from_spans(control_port.hspan().union(track_span), m2_track);
            ctx.draw_rect(m2, m2_rect);
            router.block(m2, m2_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;
            let m1_rect = control_port.with_vspan(control_port.vspan().union(m2_track));
            ctx.draw_rect(m1, m1_rect);
            router.block(m1, m1_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        // Route replica cell array to replica precharge
        for i in 0..2 {
            for port_name in ["bl", "br"] {
                let src = rbl.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                let dst = replica_pc
                    .port(PortId::new(&format!("{}_in", port_name), i))?
                    .largest_rect(m1)?;
                ctx.draw_rect(m1, src.bbox().union(dst.bbox()).into_rect());
            }
        }

        // Route replica wordline.
        let control_rwl_rect = control.port("rwl")?.largest_rect(m2)?;
        let array_rwl_rect = rbl
            .port(PortId::new("wl", dsn.rbl_wl_index))?
            .largest_rect(m2)?;
        let m1_tracks = router.track_info(m1).tracks();
        let m1_track = m1_tracks
            .index(m1_tracks.track_with_loc(TrackLocator::EndsBefore, control_rwl_rect.right()));
        let m1_rect = Rect::from_spans(
            m1_track,
            array_rwl_rect.vspan().union(control_rwl_rect.vspan()),
        );
        ctx.draw_rect(m1, m1_rect);
        router.block(m1, m1_rect);
        let m2_rect = control_rwl_rect.with_hspan(control_rwl_rect.hspan().union(m1_track));
        ctx.draw_rect(m2, m2_rect);
        router.block(m2, m2_rect);
        let via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m1, m2)
                .geometry(m1_rect, m2_rect)
                .build(),
        )?;
        ctx.draw(via)?;
        let m2_rect = array_rwl_rect.with_hspan(array_rwl_rect.hspan().union(m1_track));
        ctx.draw_rect(m2, m2_rect);
        router.block(m2, m2_rect);
        let via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m1, m2)
                .geometry(m1_rect, m2_rect)
                .build(),
        )?;
        ctx.draw(via)?;

        // Route replica bitline/precharge.
        let control_rbl_rect = control.port("rbl")?.largest_rect(m1)?;
        let control_pc_b_rect = control.port("pc_b")?.largest_rect(m1)?;
        let array_rbl_rect = replica_pc.port("rbl")?.largest_rect(m2)?;
        let array_pc_b_rect = replica_pc.port("en_b")?.largest_rect(m2)?;
        let m2_tracks = router.track_info(m2).tracks();
        let m2_track_idx =
            m2_tracks.track_with_loc(TrackLocator::StartsAfter, control_rbl_rect.bottom());
        let m2_rbl_track = m2_tracks.index(m2_track_idx);
        let m2_pc_b_track = m2_tracks.index(m2_track_idx + 1);
        let m2_wlen_track = m2_tracks.index(m2_track_idx + 2);
        let m1_tracks = router.track_info(m1).tracks();
        let m1_track_idx =
            m1_tracks.track_with_loc(TrackLocator::StartsAfter, array_rbl_rect.right());
        let m1_rbl_track = m1_tracks.index(m1_track_idx);
        let m1_pc_b_track = m1_tracks.index(m1_track_idx + 1);

        for (control_rect, array_rect, m1_track, m2_track) in [
            (control_rbl_rect, array_rbl_rect, m1_rbl_track, m2_rbl_track),
            (
                control_pc_b_rect,
                array_pc_b_rect,
                m1_pc_b_track,
                m2_pc_b_track,
            ),
        ] {
            let m1_rect = Rect::from_spans(m1_track, array_rect.vspan().union(m2_track));
            ctx.draw_rect(m1, m1_rect);
            router.block(m1, m1_rect);
            let m2_rect = array_rect.with_hspan(array_rect.hspan().union(m1_track));
            ctx.draw_rect(m2, m2_rect);
            router.block(m2, m2_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;
            let m2_rect = Rect::from_spans(m1_track.union(control_rect.hspan()), m2_track);
            ctx.draw_rect(m2, m2_rect);
            router.block(m2, m2_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;
            let m1_rect = control_rect.with_vspan(m2_track.union(control_rect.vspan()));
            ctx.draw_rect(m1, m1_rect);
            router.block(m1, m1_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        // Route wlen
        let control_wlen_rect = control.port("wlen")?.largest_rect(m1)?;
        let decoder_wlen_rect = addr_gate.port("wl_en")?.largest_rect(m1)?;

        let m2_rect = Rect::from_spans(
            control_wlen_rect.hspan().union(decoder_wlen_rect.hspan()),
            m2_wlen_track,
        );
        ctx.draw_rect(m2, m2_rect);
        router.block(m2, m2_rect);
        for rect in [control_wlen_rect, decoder_wlen_rect] {
            let m1_rect = rect.with_vspan(m2_wlen_track.union(rect.vspan()));
            ctx.draw_rect(m1, m1_rect);
            router.block(m1, m1_rect);
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(m1_rect, m2_rect)
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        // Route pc_b to main array.
        let pc_b_rect = pc_b_buffer.port("predecode_0_0")?.largest_rect(m1)?;

        let m2_rect = Rect::from_spans(
            pc_b_rect.hspan().union(control_pc_b_rect.hspan()),
            m2_pc_b_track,
        );
        ctx.draw_rect(m2, m2_rect);
        router.block(m2, m2_rect);
        let m1_rect = pc_b_rect.with_vspan(m2_pc_b_track.union(pc_b_rect.vspan()));
        ctx.draw_rect(m1, m1_rect);
        router.block(m1, m1_rect);
        let via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m1, m2)
                .geometry(m1_rect, m2_rect)
                .build(),
        )?;
        ctx.draw(via)?;

        // Route sense_en and write_driver_en.
        let m1_tracks = router.track_info(m1).tracks();
        let buffer_m1_track_idx =
            m1_tracks.track_with_loc(TrackLocator::EndsBefore, buffer_bbox.left());
        let m1_sense_en_track = m1_tracks.index(buffer_m1_track_idx);
        let m1_write_driver_en_track = m1_tracks.index(buffer_m1_track_idx - 1);

        for (port, track, buf) in [
            ("saen", m1_sense_en_track, &sense_en_buffer),
            ("wrdrven", m1_write_driver_en_track, &write_driver_en_buffer),
        ] {
            let buffer_port = buf.port("predecode_0_0")?.largest_rect(m1)?;
            let control_port = control.port(port)?.largest_rect(m2)?;

            if buffer_port.vspan().contains(control_port.vspan()) {
                let m2_rect =
                    control_port.with_hspan(control_port.hspan().union(buffer_port.hspan()));
                ctx.draw_rect(m2, m2_rect);
                router.block(m2, m2_rect);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(buffer_port, m2_rect)
                        .build(),
                )?;
                ctx.draw(via)?;
            } else {
                let m2_tracks = router.track_info(m2).tracks();
                let m2_track = m2_tracks.index(
                    if buffer_port.vspan().start() > control_port.vspan().start() {
                        m2_tracks.track_with_loc(TrackLocator::StartsAfter, buffer_port.bottom())
                    } else {
                        m2_tracks.track_with_loc(TrackLocator::EndsBefore, buffer_port.top())
                    },
                );
                let m2_rect_a = Rect::from_spans(buffer_port.hspan().union(track), m2_track);
                ctx.draw_rect(m2, m2_rect_a);
                router.block(m2, m2_rect_a);
                let m2_rect_b = control_port.with_hspan(control_port.hspan().union(track));
                ctx.draw_rect(m2, m2_rect_b);
                router.block(m2, m2_rect_b);
                if !buffer_port.vspan().intersects(&control_port.vspan()) {
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m1, m2)
                            .geometry(buffer_port, m2_rect_a)
                            .build(),
                    )?;
                    ctx.draw(via)?;
                    let m1_rect = Rect::from_spans(track, m2_track.union(control_port.vspan()));
                    ctx.draw_rect(m1, m1_rect);
                    router.block(m1, m1_rect);
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m1, m2)
                            .geometry(m1_rect, m2_rect_a)
                            .build(),
                    )?;
                    ctx.draw(via)?;
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m1, m2)
                            .geometry(m1_rect, m2_rect_b)
                            .build(),
                    )?;
                    ctx.draw(via)?;
                }
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
                let dff_port = if j == 0 {
                    let rect = dffs.port(PortId::new("q", dff_idx))?.largest_rect(m0)?;
                    let tracks = router.track_info(m1).tracks();
                    let track_span =
                        tracks.index(tracks.track_with_loc(TrackLocator::EndsBefore, rect.right()));
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m0, m1)
                            .geometry(rect.with_hspan(track_span), rect.with_hspan(track_span))
                            .build(),
                    )?;
                    let m1_rect =
                        Rect::from_spans(track_span, via.layer_bbox(m1).into_rect().vspan());
                    ctx.draw(via)?;
                    ctx.draw_rect(m1, m1_rect);
                    router.block(m1, m1_rect);
                    m1_rect
                } else {
                    let rect = dffs
                        .port(PortId::new("q_n", dff_idx))?
                        .first_rect(m0, Side::Left)?;
                    let tracks = router.track_info(m1).tracks();
                    let track_span =
                        tracks.index(tracks.track_with_loc(TrackLocator::StartsAfter, rect.left()));
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m0, m1)
                            .geometry(rect.with_hspan(track_span), rect.with_hspan(track_span))
                            .build(),
                    )?;
                    let m1_rect =
                        Rect::from_spans(track_span, via.layer_bbox(m1).into_rect().vspan());
                    ctx.draw(via)?;
                    ctx.draw_rect(m1, m1_rect);
                    router.block(m1, m1_rect);
                    m1_rect
                };

                let m2_tracks = router.track_info(m2).tracks();
                let m2_track_a = m2_tracks.index(
                    m2_tracks.track_with_loc(TrackLocator::StartsAfter, port_rect.bottom())
                        + idx as i64,
                );
                let m2_track_b = m2_tracks.index(dff_m2_track_idx + idx as i64);
                let m1_tracks = router.track_info(m1).tracks();
                let m1_track = m1_tracks.index(buffer_m1_track_idx - 2 - idx as i64);

                let m2_rect_a = Rect::from_spans(port_rect.hspan().union(m1_track), m2_track_a);
                let m1_rect_a = Rect::from_spans(m1_track, m2_track_a.union(m2_track_b));
                let m2_rect_b = Rect::from_spans(m1_track.union(dff_port.hspan()), m2_track_b);
                let m1_rect_b = dff_port.with_vspan(dff_port.vspan().union(m2_track_b));

                for (m1_rect, m2_rects) in [
                    (m1_rect_b, vec![m2_rect_b]),
                    (m1_rect_a, vec![m2_rect_a, m2_rect_b]),
                    (port_rect, vec![m2_rect_a]),
                ] {
                    ctx.draw_rect(m1, m1_rect);
                    router.block(m1, m1_rect);
                    for m2_rect in m2_rects {
                        ctx.draw_rect(m2, m2_rect);
                        router.block(m2, m2_rect);
                        let via = ctx.instantiate::<Via>(
                            &ViaParams::builder()
                                .layers(m1, m2)
                                .geometry(m1_rect, m2_rect)
                                .build(),
                        )?;
                        ctx.draw(via)?;
                    }
                }
            }
        }

        // Route row address bits to addr gate.
        for i in 0..self.params.row_bits() {
            for j in 0..2 {
                let idx = 2 * i + j;
                let dff_idx = dsn.num_dffs - i - 3 - self.params.col_select_bits();
                let port_rect = addr_gate.port(PortId::new("in", idx))?.largest_rect(m0)?;
                let dff_port = if j == 0 {
                    let rect = dffs.port(PortId::new("q", dff_idx))?.largest_rect(m0)?;
                    let tracks = router.track_info(m1).tracks();
                    let track_span =
                        tracks.index(tracks.track_with_loc(TrackLocator::EndsBefore, rect.right()));
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m0, m1)
                            .geometry(rect.with_hspan(track_span), rect.with_hspan(track_span))
                            .build(),
                    )?;
                    let m1_rect =
                        Rect::from_spans(track_span, via.layer_bbox(m1).into_rect().vspan());
                    ctx.draw(via)?;
                    ctx.draw_rect(m1, m1_rect);
                    router.block(m1, m1_rect);
                    m1_rect
                } else {
                    let rect = dffs
                        .port(PortId::new("q_n", dff_idx))?
                        .first_rect(m0, Side::Left)?;
                    let tracks = router.track_info(m1).tracks();
                    let track_span =
                        tracks.index(tracks.track_with_loc(TrackLocator::StartsAfter, rect.left()));
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m0, m1)
                            .geometry(rect.with_hspan(track_span), rect.with_hspan(track_span))
                            .build(),
                    )?;
                    let m1_rect =
                        Rect::from_spans(track_span, via.layer_bbox(m1).into_rect().vspan());
                    ctx.draw(via)?;
                    ctx.draw_rect(m1, m1_rect);
                    router.block(m1, m1_rect);
                    m1_rect
                };

                let m2_tracks = router.track_info(m2).tracks();
                let m2_track = m2_tracks.index(
                    dff_m2_track_idx + 2 * self.params.col_select_bits() as i64 + idx as i64,
                );
                let m1_tracks = router.track_info(m1).tracks();
                let m1_track_idx = m1_tracks.track_with_loc(
                    TrackLocator::EndsBefore,
                    addr_gate
                        .bbox()
                        .union(rbl.brect().expand_side(Side::Left, 6_000).bbox())
                        .into_rect()
                        .left(),
                );
                let m1_track = m1_tracks.index(m1_track_idx - idx as i64);

                let m0_rect = port_rect.with_hspan(port_rect.hspan().union(m1_track));
                let m1_rect_a = Rect::from_spans(m1_track, port_rect.vspan().union(m2_track));
                let m2_rect = Rect::from_spans(m1_track.union(dff_port.hspan()), m2_track);
                let m1_rect_b = dff_port.with_vspan(dff_port.vspan().union(m2_track));

                for (m1_rect, m2_rect) in [(m1_rect_b, m2_rect), (m1_rect_a, m2_rect)] {
                    ctx.draw_rect(m1, m1_rect);
                    router.block(m1, m1_rect);
                    ctx.draw_rect(m2, m2_rect);
                    router.block(m2, m2_rect);
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m1, m2)
                            .geometry(m1_rect, m2_rect)
                            .build(),
                    )?;
                    ctx.draw(via)?;
                }
                ctx.draw_rect(m0, m0_rect);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(m0_rect, m1_rect_a)
                        .build(),
                )?;
                ctx.draw(via)?;
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
        }
        for port_name in ["bl_dummy", "br_dummy"] {
            port_ids.push((PortId::new(port_name, 1), SingleSupplyNet::Vdd));
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
                    for sign in [Sign::Neg, Sign::Pos] {
                        let rect = port.with_hspan(Span::new(
                            new_span.point(sign),
                            port.hspan().point(sign) - sign.as_int() * 800,
                        ));
                        if layer == m1 {
                            let via = ctx.instantiate::<Via>(
                                &ViaParams::builder()
                                    .layers(m1, m2)
                                    .geometry(port, rect)
                                    .build(),
                            )?;
                            ctx.draw(via)?;
                        }
                        router.block(m2, rect);
                        straps.add_target(
                            layer,
                            Target::new(
                                match port_name {
                                    "vdd" => SingleSupplyNet::Vdd,
                                    "vss" => SingleSupplyNet::Vss,
                                    _ => unreachable!(),
                                },
                                rect,
                            ),
                        );
                        ctx.draw_rect(m2, rect);
                    }
                }
            }
        }

        // Connect decoders and DFFs to power straps.
        for (inst, layer) in [
            (&decoder, m1),
            (&addr_gate, m2),
            (&col_dec, m2),
            (&dffs, m2),
            (&control, m1),
        ] {
            for port_name in ["vdd", "vss"] {
                for port in inst.port(port_name)?.shapes(layer) {
                    if let Shape::Rect(rect) = port {
                        straps.add_target(
                            layer,
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

        // Connect replica precharge to power straps.
        for port in replica_pc.port("vdd")?.shapes(m2) {
            if let Shape::Rect(rect) = port {
                straps.add_target(m2, Target::new(SingleSupplyNet::Vdd, rect));
            }
        }

        // Route column peripheral outputs to pins on bounding box of SRAM
        let groups = self.params.data_width();
        for (port, width) in [
            ("dout", groups),
            ("din", groups),
            ("wmask", self.params.wmask_width()),
        ] {
            for i in 0..width {
                let port_id = PortId::new(port, i);
                let rect = cols.port(port_id.clone())?.largest_rect(m1)?;
                let rect = rect.with_vspan(rect.vspan().add_point(router_bbox.bottom()));
                ctx.draw_rect(m1, rect);
                ctx.add_port(CellPort::builder().id(port_id).add(m1, rect).build())?;
                router.block(m1, rect);
            }
        }

        let straps = straps.fill(&router, ctx)?;
        ctx.set_metadata(straps);

        ctx.draw(router)?;
        Ok(())
    }
}
