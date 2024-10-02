use std::collections::{HashMap, VecDeque};

use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Named;
use subgeom::{Corner, Dir, Point, Rect, Shape, Side, Sign, Span};
use substrate::component::{Component, NoParams};
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Instance, Port, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
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
use crate::blocks::decoder::layout::LastBitDecoderStage;
use crate::blocks::decoder::{
    AddrGate, AddrGateParams, DecoderParams, DecoderStageParams, DecoderTree, Predecoder,
    WmuxDriver, INV_PARAMS, NAND2_PARAMS,
};
use crate::blocks::gate::{AndParams, GateParams};
use crate::blocks::precharge::layout::{ReplicaPrecharge, ReplicaPrechargeParams};

use super::schematic::fanout_buffer_stage;
use super::SramInner;

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

impl SramInner {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let col_params = self.col_params();
        let bitcells = ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows(),
            cols: self.params.cols(),
            mux_ratio: self.params.mux_ratio(),
        })?;
        let mut cols = ctx.instantiate::<ColPeripherals>(&col_params)?;
        // TODO
        let tree = DecoderTree::new(self.params.row_bits(), 128e-15);
        let mut decoder = ctx
            .instantiate::<Predecoder>(&DecoderParams {
                max_width: None,
                tree,
            })?
            .with_orientation(Named::R90Cw);
        let mut addr_gate = ctx
            .instantiate::<AddrGate>(&AddrGateParams {
                // TODO fix, should be minimum sized AND2 unless sized elsewhere
                gate: GateParams::And2(AndParams {
                    nand: NAND2_PARAMS,
                    inv: INV_PARAMS,
                }),
                num: 2 * self.params.row_bits(),
            })?
            .with_orientation(Named::FlipYx);

        let col_tree = DecoderTree::new(self.params.col_select_bits(), 128e-15); // TODO
        let col_decoder_params = DecoderParams {
            max_width: Some(
                cols.port(PortId::new("sel_b", self.params.col_select_bits() - 1))?
                    .largest_rect(m2)?
                    .vspan()
                    .union(
                        cols.port(PortId::new("sel", self.params.col_select_bits() - 1))?
                            .largest_rect(m2)?
                            .vspan(),
                    )
                    .length(),
            ),
            tree: col_tree.clone(),
        };

        let mut col_dec = ctx
            .instantiate::<Predecoder>(&col_decoder_params)?
            .with_orientation(Named::R90Cw);
        let mut control = ctx.instantiate::<ControlLogicReplicaV2>(&NoParams)?;

        let pc_b_buffer = DecoderStageParams {
            max_width: Some(
                cols.port("pc_b")?
                    .largest_rect(m2)?
                    .vspan()
                    .add_point(cols.brect().top())
                    .length(),
            ),
            ..fanout_buffer_stage(50e-15)
        };
        let mut pc_b_buffer = ctx
            .instantiate::<LastBitDecoderStage>(&pc_b_buffer)?
            .with_orientation(Named::R90Cw);
        let wrdrven_saen_width = cols
            .port("sense_en")?
            .largest_rect(m2)?
            .vspan()
            .add_point(cols.brect().bottom())
            .length()
            / 2;
        let write_driver_en_buffer = DecoderStageParams {
            max_width: Some(wrdrven_saen_width),
            ..fanout_buffer_stage(50e-15)
        };
        let mut write_driver_en_buffer = ctx
            .instantiate::<LastBitDecoderStage>(&write_driver_en_buffer)?
            .with_orientation(Named::R90Cw);
        let sense_en_buffer = DecoderStageParams {
            max_width: Some(wrdrven_saen_width),
            ..fanout_buffer_stage(50e-15)
        };
        let mut sense_en_buffer = ctx
            .instantiate::<LastBitDecoderStage>(&sense_en_buffer)?
            .with_orientation(Named::R90Cw);

        // TODO: decide how registers should be organized
        let num_dffs = self.params.addr_width() + 2;
        let mut dffs = ctx.instantiate::<DffArray>(&num_dffs)?;

        let rbl_rows = ((self.params.rows() / 12) + 1) * 2;
        let rbl_wl_index = rbl_rows / 2;
        let mut rbl = ctx.instantiate::<ReplicaCellArray>(&ReplicaCellArrayParams {
            rows: rbl_rows,
            cols: 2,
        })?;
        let mut replica_pc = ctx.instantiate::<ReplicaPrecharge>(&ReplicaPrechargeParams {
            cols: 2,
            inner: col_params.pc,
        })?;

        cols.align_beneath(bitcells.bbox(), 4_000);
        cols.align_centers_horizontally_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_to_the_left_of(bitcells.bbox(), 6_000);
        decoder.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        pc_b_buffer.align_bottom(cols.port("pc_b")?.largest_rect(m2)?);
        pc_b_buffer.align_to_the_left_of(cols.bbox(), 6_000);
        col_dec.align_bottom(
            cols.port(PortId::new("sel_b", self.params.col_select_bits() - 1))?
                .largest_rect(m2)?,
        );
        col_dec.align_to_the_left_of(cols.bbox(), 6_000);
        sense_en_buffer.align_top(cols.port("sense_en")?.largest_rect(m2)?);
        sense_en_buffer.align_to_the_left_of(cols.bbox(), 6_000);
        write_driver_en_buffer.align_beneath(sense_en_buffer.bbox(), 4_000);
        write_driver_en_buffer.align_to_the_left_of(cols.bbox(), 6_000);
        addr_gate.align_to_the_left_of(decoder.bbox(), 4_000);
        addr_gate.align_bottom(decoder.bbox());
        rbl.align_to_the_left_of(
            col_dec
                .bbox()
                .union(sense_en_buffer.bbox())
                .union(pc_b_buffer.bbox())
                .union(write_driver_en_buffer.bbox()),
            6_000,
        );
        rbl.align_beneath(decoder.bbox(), 6_000);
        control.set_orientation(Named::FlipYx);
        control.align_beneath(decoder.bbox(), 4_000);
        control.align_to_the_left_of(rbl.bbox(), 6_000);
        replica_pc.align_beneath(rbl.bbox(), 4_000);
        replica_pc.align_centers_horizontally_gridded(rbl.bbox(), ctx.pdk().layout_grid());
        dffs.align_right(rbl.bbox());
        dffs.align_beneath(control.bbox().union(replica_pc.bbox()), 6_000);

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

        // Route precharges to bitcell array
        for i in 0..self.params.cols() {
            for port_name in ["bl", "br"] {
                let src = cols.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                let dst = bitcells.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                ctx.draw_rect(m1, src.union(dst.bbox()).into_rect());
            }
        }

        let router_bbox = ctx
            .brect()
            .expand(8 * 680)
            .expand_side(Side::Right, 4 * 680)
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

        for inst in [&bitcells, &rbl, &cols] {
            router.block(m1, inst.brect().expand_dir(Dir::Vert, 6_000));
            router.block(m2, inst.brect().expand_dir(Dir::Horiz, 6_000));
        }

        // Route DFF input signals to pins on bounding box of SRAM
        for i in 0..num_dffs {
            let src = dffs.port(PortId::new("d", i))?.largest_rect(m0)?;
            let expanded_rect =
                router.expand_to_layer_grid(src, m1, ExpandToGridStrategy::Side(Side::Bot));
            ctx.draw_rect(m1, expanded_rect);
            let tracks = router.track_info(m1).tracks();
            let track_span =
                tracks.index(tracks.track_with_loc(TrackLocator::EndsBefore, src.right()));
            let m1_rect = src
                .with_hspan(track_span)
                .with_vspan(src.vspan().add_point(router_bbox.bottom()));
            ctx.draw_rect(m1, m1_rect);
            ctx.add_port(
                CellPort::builder()
                    .id(if i == num_dffs - 1 {
                        "we".into()
                    } else if i == num_dffs - 2 {
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
                    .geometry(src, expanded_rect)
                    .build(),
            )?;
            ctx.draw(via)?;
            router.block(m1, expanded_rect);
            router.block(m1, m1_rect);
        }

        // Route address gate to predecoders.
        for i in 0..self.params.row_bits() {
            for j in 0..2 {
                let idx = 2 * i + j;
                let y = addr_gate.port(PortId::new("y", idx))?.largest_rect(m0)?;
                let predecode_port = decoder
                    .port(format!("predecode_{}_{}", i, j))?
                    .largest_rect(m1)?;
                let tracks = router.track_info(m2).tracks();
                let track_idx = tracks.track_with_loc(TrackLocator::Nearest, y.top());
                let track = tracks.index(track_idx);

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
                let m1_rect = via.layer_bbox(m1).into_rect();
                let m1_rect = m1_rect.with_vspan(m1_rect.vspan().union(track));
                let m2_rect =
                    Rect::from_spans(m1_rect.hspan().union(predecode_port.hspan()), track);
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

                // Route on m2 to the appropriate predecoder port and via down.
                let vspan = if track.start() > predecode_port.vspan().stop() {
                    Span::new(predecode_port.vspan().stop() - 320, track.stop())
                } else if track.stop() < predecode_port.vspan().start() {
                    Span::new(track.start(), predecode_port.vspan().start() + 320)
                } else {
                    track
                };
                let vert_m2_rect = Rect::from_spans(predecode_port.hspan(), vspan);
                ctx.draw_rect(m2, vert_m2_rect);
                router.block(m2, vert_m2_rect);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(predecode_port, vert_m2_rect)
                        .build(),
                )?;
                ctx.draw_ref(&via)?;
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
            let track_idx = tracks.track_with_loc(TrackLocator::StartsAfter, y.right());
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

        // Route column decoders to mux.
        let tracks = router.track_info(m1).tracks();
        let track_idx = tracks.track_with_loc(TrackLocator::StartsAfter, col_dec.brect().right());
        for i in 0..self.params.col_select_bits() {
            for j in 0..2 {
                let (y_name, sel_name) = if j == 0 {
                    ("y", "sel")
                } else {
                    ("y_b", "sel_b")
                };
                let y = addr_gate.port(PortId::new(y_name, i))?.largest_rect(m0)?;
                let sel = decoder.port(PortId::new(sel_name, i))?.largest_rect(m1)?;
                let track_rect =
                    Rect::from_spans(tracks.index(track_idx + i), y.vspan().union(sel.vspan()));

                let rect = if j == 0 {
                    y.with_hspan(y.hspan().union(track_rect.hspan()))
                } else {
                    let jog = OffsetJog::builder()
                        .dir(subgeom::Dir::Horiz)
                        .sign(subgeom::Sign::Pos)
                        .src(y)
                        .dst(y.top() + 340)
                        .layer(m0)
                        .space(170)
                        .build()
                        .unwrap();
                    let rect = Rect::from_spans(
                        jog.r2().hspan().union(track_rect.hspan()),
                        Span::with_start_and_length(y.top() + 170, 170),
                    );
                    ctx.draw(jog)?;
                    rect
                };
                ctx.draw_rect(m0, rect);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(rect, track_rect)
                        .build(),
                )?;
                ctx.draw_ref(&via)?;
                let m2_rect = Rect::from_spans(track_rect.hspan().union(sel.hspan()), sel.vspan());
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

                // Route on m2 to the appropriate predecoder port and via down.
                let vspan = if track.start() > predecode_port.vspan().stop() {
                    Span::new(predecode_port.vspan().stop() - 320, track.stop())
                } else if track.stop() < predecode_port.vspan().start() {
                    Span::new(track.start(), predecode_port.vspan().start() + 320)
                } else {
                    track
                };
                let vert_m2_rect = Rect::from_spans(predecode_port.hspan(), vspan);
                ctx.draw_rect(m2, vert_m2_rect);
                router.block(m2, vert_m2_rect);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(predecode_port, vert_m2_rect)
                        .build(),
                )?;
                ctx.draw_ref(&via)?;
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
        for i in 0..rbl_rows {
            if i != rbl_wl_index {
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
        let straps = straps.fill(&router, ctx)?;
        ctx.set_metadata(straps);

        ctx.draw(router)?;
        Ok(())
    }
}
