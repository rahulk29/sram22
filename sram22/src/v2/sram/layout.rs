use std::collections::{HashMap, VecDeque};

use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Named;
use subgeom::{Corner, Dir, Point, Rect, Shape, Side, Sign, Span};
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Instance, Port, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::routing::auto::grid::{
    ExpandToGridStrategy, JogToGrid, OffGridBusTranslation, OffGridBusTranslationStrategy,
};
use substrate::layout::routing::auto::straps::{RoutedStraps, Target};
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::{ElbowJog, SJog};
use substrate::layout::routing::tracks::{TrackLocator, UniformTracks};
use substrate::layout::straps::SingleSupplyNet;
use substrate::layout::Draw;

use crate::v2::bitcell_array::replica::{ReplicaCellArray, ReplicaCellArrayParams};
use crate::v2::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::v2::columns::ColPeripherals;
use crate::v2::control::{ControlLogicKind, ControlLogicReplicaV2, DffArray};
use crate::v2::decoder::layout::LastBitDecoderStage;
use crate::v2::decoder::{
    AddrGate, AddrGateParams, DecoderParams, DecoderStageParams, DecoderTree, Predecoder,
    WmuxDriver,
};
use crate::v2::gate::GateParams;
use crate::v2::precharge::layout::{ReplicaPrecharge, ReplicaPrechargeParams};

use super::{ControlMode, SramInner};

impl SramInner {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let col_params = self.col_params();
        let bitcells = ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows,
            cols: self.params.cols,
            mux_ratio: self.params.mux_ratio,
        })?;
        let mut cols = ctx.instantiate::<ColPeripherals>(&col_params)?;
        let tree = DecoderTree::with_scale_and_skew(self.params.row_bits, 2, false);
        let decoder_params = DecoderStageParams {
            gate: tree.root.gate,
            num: tree.root.num,
            child_sizes: tree.root.children.iter().map(|n| n.num).collect(),
        };
        let mut decoder = ctx
            .instantiate::<LastBitDecoderStage>(&decoder_params)?
            .with_orientation(Named::R90Cw);
        let mut addr_gate = ctx.instantiate::<AddrGate>(&AddrGateParams {
            gate: tree.root.gate,
            num: 2 * self.params.row_bits,
        })?;

        let mut p1 = ctx.instantiate::<Predecoder>(&DecoderParams {
            tree: DecoderTree {
                root: tree.root.children[0].clone(),
            },
        })?;
        let mut p2 = ctx.instantiate::<Predecoder>(&DecoderParams {
            tree: DecoderTree {
                root: tree.root.children[1].clone(),
            },
        })?;
        let p1_bits = tree.root.children[0].num.ilog2() as usize;
        let p2_bits = tree.root.children[1].num.ilog2() as usize;

        let col_tree = DecoderTree::for_columns(self.params.col_select_bits, 1);
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };

        let mut col_dec = ctx.instantiate::<Predecoder>(&col_decoder_params)?;
        let wmux_driver_params = DecoderStageParams {
            gate: col_tree.root.gate,
            num: col_tree.root.num,
            child_sizes: vec![],
        };
        let mut wmux_driver = ctx.instantiate::<WmuxDriver>(&wmux_driver_params)?;
        let mut control = ctx.instantiate::<ControlLogicReplicaV2>(&match self.params.control {
            ControlMode::ReplicaV2 => ControlLogicKind::Standard,
            ControlMode::ReplicaV2Test => ControlLogicKind::Test,
        })?;

        let num_dffs = self.params.addr_width + 1;
        let mut dffs = ctx.instantiate::<DffArray>(&num_dffs)?;

        let rbl_rows = ((self.params.rows / 12) + 1) * 2;
        let rbl_wl_index = rbl_rows / 2;
        let mut rbl = ctx.instantiate::<ReplicaCellArray>(&ReplicaCellArrayParams {
            rows: rbl_rows,
            cols: 2,
        })?;
        let mut replica_pc = ctx.instantiate::<ReplicaPrecharge>(&ReplicaPrechargeParams {
            cols: 2,
            inner: col_params.pc,
        })?;

        cols.align_beneath(bitcells.bbox(), 4_310);
        cols.align_centers_horizontally_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_to_the_left_of(bitcells.bbox(), 10_000);
        decoder.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_side_to_grid(Side::Left, 680);
        rbl.align_to_the_left_of(decoder.bbox(), 4_310);
        rbl.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        rbl.align_side_to_grid(Side::Left, 680);
        replica_pc.align_beneath(rbl.bbox(), 5_080);
        replica_pc.align_centers_horizontally_gridded(rbl.bbox(), ctx.pdk().layout_grid());
        p1.align_beneath(
            decoder.bbox(),
            5_080 * (tree.root.children[0].num + tree.root.children[1].num) as i64 / 16 + 3_000,
        );
        p1.align_right(decoder.bbox());
        p2.align_beneath(p1.bbox(), 5_080);
        p2.align_right(decoder.bbox());
        wmux_driver.align_beneath(p2.bbox(), 5_080);
        wmux_driver.align_right(decoder.bbox());
        col_dec.align_beneath(wmux_driver.bbox(), 5_080);
        col_dec.align_right(decoder.bbox());
        addr_gate.align_bottom(col_dec.bbox());
        addr_gate.align_to_the_left_of(col_dec.bbox(), 6_350);
        control.set_orientation(Named::FlipYx);
        control.align_beneath(col_dec.bbox(), 10_000);
        control.align_right(decoder.bbox());
        dffs.align_beneath(control.bbox(), 10_000);
        dffs.align_right(decoder.bbox());

        ctx.draw_ref(&bitcells)?;
        ctx.draw_ref(&cols)?;
        ctx.draw_ref(&decoder)?;
        ctx.draw_ref(&addr_gate)?;
        ctx.draw_ref(&wmux_driver)?;
        ctx.draw_ref(&p1)?;
        ctx.draw_ref(&p2)?;
        ctx.draw_ref(&col_dec)?;
        ctx.draw_ref(&control)?;
        ctx.draw_ref(&dffs)?;
        ctx.draw_ref(&rbl)?;
        ctx.draw_ref(&replica_pc)?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

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
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        for inst in [
            &p1,
            &p2,
            &addr_gate,
            &col_dec,
            &wmux_driver,
            &dffs,
            &control,
            &cols,
            &replica_pc,
        ] {
            for layer in [m1, m2, m3] {
                for shape in inst.shapes_on(layer) {
                    let rect = shape.brect();
                    router.block(layer, rect);
                }
            }
        }

        for inst in [&bitcells, &rbl] {
            router.block(m1, inst.brect());
            router.block(m2, inst.brect().expand_dir(Dir::Horiz, 6_000));
            router.block(m3, inst.brect());
        }

        let mut to_route = Vec::new();

        // Route DFF input signals to pins on bounding box of SRAM
        for i in 0..num_dffs {
            let src = dffs.port(PortId::new("d", i))?.largest_rect(m2)?;
            let src = router.expand_to_layer_grid(src, m2, ExpandToGridStrategy::Minimum);
            let src = router.expand_to_layer_grid(src, m3, ExpandToGridStrategy::Minimum);
            ctx.draw_rect(m2, src);
            let tracks = router.track_info(m3).tracks();
            let track_span =
                tracks.index(tracks.track_with_loc(TrackLocator::EndsBefore, src.right()));
            let rect = src
                .with_hspan(track_span)
                .with_vspan(src.vspan().add_point(router_bbox.bottom()));
            ctx.draw_rect(m3, rect);
            ctx.add_port(
                CellPort::builder()
                    .id(if i == num_dffs - 1 {
                        "we".into()
                    } else {
                        PortId::new("addr", self.params.addr_width - i - 1)
                    })
                    .add(m3, rect)
                    .build(),
            )?;
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m2, m3)
                    .geometry(src, rect)
                    .build(),
            )?;
            ctx.draw(via)?;
            router.block(m3, rect);
        }

        for i in 0..2 * self.params.row_bits {
            let src = dffs
                .port(PortId::new(if i % 2 == 0 { "q" } else { "qn" }, i / 2))?
                .largest_rect(m2)?;
            let src = router.expand_to_grid(
                src,
                ExpandToGridStrategy::Side(if i % 2 == 0 { Side::Top } else { Side::Bot }),
            );
            ctx.draw_rect(m2, src);
            router.block(m2, src);
            let dst = addr_gate.port(PortId::new("in", i))?.largest_rect(m0)?;
            let dst = router.register_jog_to_grid(
                JogToGrid::builder()
                    .layer(m0)
                    .rect(dst)
                    .dst_layer(m1)
                    .width(170)
                    .first_dir(Side::Bot)
                    .second_dir(if i % 2 == 0 { Side::Right } else { Side::Left })
                    .build(),
            );
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(dst, dst)
                    .build(),
            )?;
            ctx.draw(via)?;
            to_route.push((m2, src, m1, dst, None));
        }

        // Route address bits from DFFs to decoders
        let mut ctr = 0;
        for (inst, subtree) in [
            (&p1, &tree.root.children[0]),
            (&p2, &tree.root.children[1]),
            (&col_dec, &col_tree.root),
        ] {
            let bits = subtree.num.ilog2() as usize;
            let mut all_ports = Vec::new();

            let mut queue = VecDeque::new();
            queue.push_front(subtree);
            let mut total = 0;
            while let Some(node) = queue.pop_front() {
                if node.children.is_empty() {
                    println!("total: {total}");
                    let curr_bits = node.num.ilog2();
                    total += curr_bits;
                    println!("total: {total}");
                    let bottom_port = inst
                        .port(&format!("predecode_{}_1", total - 1))?
                        .largest_rect(m2)?;
                    let on_grid_bus = router.register_off_grid_bus_translation(
                        ctx,
                        OffGridBusTranslation::builder()
                            .strategy(OffGridBusTranslationStrategy::Perpendicular(m3))
                            .layer(m2)
                            .line_and_space(320, 160)
                            .output(bottom_port.edge(Side::Bot))
                            .start(bottom_port.side(Side::Bot))
                            .n(2 * curr_bits as i64)
                            .build(),
                    )?;

                    let mut ports: Vec<Rect> = on_grid_bus.ports().collect();
                    ports.reverse();
                    all_ports.extend(ports);
                } else {
                    for child in node.children.iter().rev() {
                        queue.push_front(child);
                    }
                }
            }

            for i in 0..bits {
                for j in 0..2 {
                    let (layer, src) = if ctr < p1_bits + p2_bits {
                        let src = addr_gate
                            .port(PortId::new("decode", 2 * ctr + j))?
                            .largest_rect(m0)?;
                        let src = router.register_jog_to_grid(
                            JogToGrid::builder()
                                .layer(m0)
                                .rect(src)
                                .dst_layer(m1)
                                .width(170)
                                .first_dir(Side::Top)
                                .second_dir(if j == 0 { Side::Right } else { Side::Left })
                                .build(),
                        );
                        let via = ctx.instantiate::<Via>(
                            &ViaParams::builder()
                                .layers(m0, m1)
                                .geometry(src, src)
                                .build(),
                        )?;
                        ctx.draw(via)?;
                        (m1, src)
                    } else {
                        let src = dffs
                            .port(PortId::new(if j == 0 { "q" } else { "qn" }, ctr))?
                            .largest_rect(m2)?;
                        let src = router.expand_to_grid(
                            src,
                            ExpandToGridStrategy::Side(if j == 0 { Side::Top } else { Side::Bot }),
                        );
                        ctx.draw_rect(m2, src);
                        router.block(m2, src);
                        (m2, src)
                    };
                    let dst = all_ports[2 * i + j];
                    to_route.push((layer, src, m3, dst, None));
                }
                ctr += 1;
            }
        }

        let left_port = decoder
            .port(&format!("predecode_1_{}", tree.root.children[1].num - 1))?
            .largest_rect(m1)?;

        let decoder_tracks = UniformTracks::builder()
            .line(320)
            .space(160)
            .start(left_port.side(Side::Left))
            .sign(Sign::Pos)
            .build()
            .unwrap();
        for i in 0..(tree.root.children[0].num + tree.root.children[1].num) {
            ctx.draw_rect(
                m1,
                Rect::from_spans(
                    decoder_tracks.index(i),
                    Span::with_stop_and_length(left_port.side(Side::Bot), 140),
                ),
            );
        }
        let decoder_bus = router.register_off_grid_bus_translation(
            ctx,
            OffGridBusTranslation::builder()
                .strategy(OffGridBusTranslationStrategy::Parallel)
                .layer(m1)
                .line_and_space(320, 160)
                .output(left_port.edge(Side::Bot).offset(140))
                .start(left_port.side(Side::Left))
                .n((tree.root.children[0].num + tree.root.children[1].num) as i64)
                .build(),
        )?;

        let mut decoder_ports = decoder_bus.ports().collect::<Vec<Rect>>();
        decoder_ports.reverse();

        let (p0_ports, p1_ports) = decoder_ports.split_at(tree.root.children[0].num);

        // Route predecoders to final decoder stage
        for (i, &dst) in p0_ports.iter().enumerate().take(tree.root.children[0].num) {
            let src = p1.port(PortId::new("decode", i))?.largest_rect(m0)?;
            let src = router.register_jog_to_grid(
                JogToGrid::builder()
                    .layer(m0)
                    .rect(src)
                    .dst_layer(m1)
                    .width(170)
                    .first_dir(Side::Top)
                    .second_dir(if i % 2 == 0 { Side::Right } else { Side::Left })
                    .build(),
            );
            to_route.push((m1, src, m1, dst, None));
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(src, src)
                    .build(),
            )?;
            ctx.draw(via)?;
        }
        for (i, &dst) in p1_ports.iter().enumerate().take(tree.root.children[1].num) {
            let src = p2.port(PortId::new("decode", i))?.largest_rect(m0)?;
            let src = router.register_jog_to_grid(
                JogToGrid::builder()
                    .layer(m0)
                    .rect(src)
                    .dst_layer(m1)
                    .width(170)
                    .first_dir(Side::Top)
                    .second_dir(if i % 2 == 0 { Side::Right } else { Side::Left })
                    .build(),
            );
            to_route.push((m1, src, m1, dst, None));
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(src, src)
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        // Route write mux driver to write muxes.
        let bottom_port = cols
            .port(PortId::new("we", self.params.mux_ratio - 1))?
            .largest_rect(m2)?;
        let on_grid_bus = router.register_off_grid_bus_translation(
            ctx,
            OffGridBusTranslation::builder()
                .strategy(OffGridBusTranslationStrategy::Parallel)
                .layer(m2)
                .line_and_space(340, 160)
                .output(bottom_port.edge(Side::Left))
                .start(bottom_port.side(Side::Bot))
                .n(self.params.mux_ratio as i64)
                .build(),
        )?;
        for (i, dst) in on_grid_bus.ports().enumerate() {
            let src = wmux_driver
                .port(PortId::new("decode", i))?
                .largest_rect(m0)?;
            let src = router.register_jog_to_grid(
                JogToGrid::builder()
                    .layer(m0)
                    .rect(src)
                    .dst_layer(m1)
                    .width(170)
                    .first_dir(Side::Top)
                    .second_dir(if i % 2 == 0 { Side::Right } else { Side::Left })
                    .build(),
            );
            to_route.push((m1, src, m2, dst, None));
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(src, src)
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        // Route column decoder to read muxes.
        let bottom_port = cols
            .port(PortId::new("sel_b", self.params.mux_ratio - 1))?
            .largest_rect(m2)?;
        let on_grid_bus = router.register_off_grid_bus_translation(
            ctx,
            OffGridBusTranslation::builder()
                .strategy(OffGridBusTranslationStrategy::Parallel)
                .layer(m2)
                .line_and_space(320, 180)
                .output(bottom_port.edge(Side::Left))
                .start(bottom_port.side(Side::Bot))
                .n(self.params.mux_ratio as i64)
                .shift(-1)
                .build(),
        )?;
        let and3_dec = matches!(col_tree.root.gate, GateParams::And3(_));
        for (i, dst) in on_grid_bus.ports().enumerate() {
            let src = col_dec.port(PortId::new("decode_b", i))?.largest_rect(m0)?;
            let src = router.register_jog_to_grid(
                JogToGrid::builder()
                    .layer(m0)
                    .rect(src)
                    .dst_layer(m1)
                    .width(170)
                    .first_dir(if !and3_dec && i % 2 == 0 {
                        Side::Top
                    } else {
                        Side::Bot
                    })
                    .second_dir({
                        let side = if i % 2 == 0 { Side::Right } else { Side::Left };
                        if and3_dec {
                            !side
                        } else {
                            side
                        }
                    })
                    .build(),
            );
            to_route.push((m2, dst, m1, src, None));
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(src, src)
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        // Route wordline driver to bitcell array
        for i in 0..tree.root.num {
            let src = decoder.port(PortId::new("decode", i))?.largest_rect(m0)?;
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
            ctx.draw(via)?;
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(src, src)
                    .build(),
            )?;
            ctx.draw(via)?;
            ctx.draw(jog_group)?;
        }

        // Route column decoder to wmux driver
        for i in 0..col_tree.root.num {
            let src = col_dec.port(PortId::new("decode", i))?.largest_rect(m0)?;
            let dst = wmux_driver.port(PortId::new("in", i))?.largest_rect(m0)?;
            let jog = SJog::builder()
                .src(src)
                .dst(dst)
                .dir(Dir::Vert)
                .layer(m0)
                .width(170)
                .grid(ctx.pdk().layout_grid())
                .build()
                .unwrap();
            ctx.draw(jog)?;
        }

        // Route precharges to bitcell array
        for i in 0..self.params.cols {
            for port_name in ["bl", "br"] {
                let src = cols.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                let dst = bitcells.port(PortId::new(port_name, i))?.largest_rect(m1)?;
                ctx.draw_rect(m1, src.union(dst.bbox()).into_rect());
            }
        }

        for (cols_port_name, bitcell_port_name) in
            [("dummy_bl_in", "bl_dummy"), ("dummy_br_in", "br_dummy")]
        {
            let src = cols.port(cols_port_name)?.largest_rect(m1)?;
            let dst = bitcells.port(bitcell_port_name)?.largest_rect(m1)?;
            ctx.draw_rect(m1, src.union(dst.bbox()).into_rect());
        }

        // Route control logic inputs.
        let src = control.port("we")?.largest_rect(m1)?;
        let src = router.expand_to_grid(src, ExpandToGridStrategy::Side(Side::Bot));
        ctx.draw_rect(m1, src);

        let dst = dffs
            .port(PortId::new("q", num_dffs - 1))?
            .largest_rect(m2)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Minimum);
        ctx.draw_rect(m2, dst);

        to_route.push((m1, src, m2, dst, Some("we")));

        // Route clock signal
        let tracks = router.track_info(m3).tracks().clone();
        let start_track = tracks.track_with_loc(TrackLocator::StartsAfter, dffs.brect().right());

        let track_span = tracks.index(start_track);
        let clk_pin = Rect::from_spans(
            track_span,
            Span::with_start_and_length(router_bbox.bottom(), 1000),
        );
        ctx.draw_rect(m3, clk_pin);
        ctx.add_port(CellPort::with_shape("clk", m3, clk_pin))?;
        router.occupy(m3, clk_pin, "clk")?;

        let src = control.port("clk")?.largest_rect(m1)?;
        let src = router.expand_to_grid(src, ExpandToGridStrategy::Side(Side::Bot));
        ctx.draw_rect(m1, src);
        to_route.push((m1, src, m3, clk_pin, Some("clk")));

        if matches!(self.params.control, ControlMode::ReplicaV2Test) {
            for (i, port_name) in ["sae_int", "sae_muxed"].into_iter().enumerate() {
                let track_span = tracks.index(start_track + i as i64 + 1);
                let pin = Rect::from_spans(
                    track_span,
                    Span::with_start_and_length(router_bbox.bottom(), 1000),
                );
                ctx.draw_rect(m3, pin);
                ctx.add_port(CellPort::with_shape(port_name, m3, pin))?;
                router.occupy(m3, pin, port_name)?;

                let src = control.port(port_name)?.largest_rect(m1)?;
                let src = router.expand_to_grid(src, ExpandToGridStrategy::Side(Side::Bot));
                ctx.draw_rect(m1, src);
                to_route.push((m1, src, m3, pin, Some(port_name)));
            }
        }

        for i in 0..num_dffs {
            let src = dffs.port(PortId::new("clk", i))?.largest_rect(m2)?;
            let src = router.expand_to_grid(src, ExpandToGridStrategy::Side(Side::Top));
            ctx.draw_rect(m2, src);
            router.occupy(m2, src, "clk")?;
            to_route.push((m2, src, m3, clk_pin, Some("clk")));
        }

        for i in (0..2).rev() {
            let mut clk_bbox = Bbox::empty();
            for shape in cols.port(PortId::new("clk", i))?.shapes(m2) {
                clk_bbox = clk_bbox.union(shape.bbox());
            }
            let clk_rect = clk_bbox.into_rect();
            ctx.draw_rect(m2, clk_rect);

            let expanded_all = router.expand_to_grid(clk_rect, ExpandToGridStrategy::All);
            let src = clk_rect.with_hspan(Span::new(expanded_all.left(), clk_rect.left()));
            let src = router.expand_to_grid(src, ExpandToGridStrategy::Corner(Corner::UpperLeft));
            ctx.draw_rect(m2, src);
            router.occupy(m2, src, "clk")?;
            to_route.push((m2, src, m3, clk_pin, Some("clk")));
        }

        // Route replica precharge to control logic.
        let bottom_port = replica_pc.port("rbr")?.largest_rect(m2)?;
        let ports = router
            .register_off_grid_bus_translation(
                ctx,
                OffGridBusTranslation::builder()
                    .strategy(OffGridBusTranslationStrategy::Parallel)
                    .layer(m2)
                    .line_and_space(360, 140)
                    .output(bottom_port.edge(Side::Left))
                    .start(bottom_port.side(Side::Bot))
                    .n(3)
                    .shift(-1)
                    .build(),
            )?
            .ports()
            .collect::<Vec<Rect>>();

        let replica_pc_en_b_rect = ports[2];
        let rbl_rect = ports[1];

        // Route control logic outputs
        let pc_b_rect = cols.port("pc_b")?.largest_rect(m2)?;
        let expanded_all = router.expand_to_grid(pc_b_rect, ExpandToGridStrategy::All);
        let src = pc_b_rect.with_hspan(Span::new(expanded_all.left(), pc_b_rect.left()));
        let src = router.expand_to_grid(src, ExpandToGridStrategy::Corner(Corner::UpperLeft));
        ctx.draw_rect(m2, src);
        router.occupy(m2, src, "pc_b")?;

        let dst = control.port("pc_b")?.largest_rect(m1)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Corner(Corner::UpperRight));
        ctx.draw_rect(m1, dst);

        to_route.push((m2, src, m1, dst, Some("pc_b")));
        to_route.push((m2, replica_pc_en_b_rect, m1, dst, Some("pc_b")));

        let dst = control.port("rbl")?.largest_rect(m1)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Side(Side::Top));
        ctx.draw_rect(m1, dst);

        to_route.push((m2, rbl_rect, m1, dst, Some("rbl")));

        let sense_en_rect = cols.port("sense_en")?.largest_rect(m2)?;
        let expanded_all = router.expand_to_grid(sense_en_rect, ExpandToGridStrategy::All);
        let src = sense_en_rect.with_hspan(Span::new(expanded_all.left(), sense_en_rect.left()));
        let src = router.expand_to_grid(src, ExpandToGridStrategy::Corner(Corner::LowerLeft));
        ctx.draw_rect(m2, src);
        router.occupy(m2, src, "sense_en")?;

        let dst = control.port("sense_en")?.largest_rect(m1)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Side(Side::Top));
        ctx.draw_rect(m1, dst);

        to_route.push((m2, src, m1, dst, Some("sense_en")));

        let wl_en_rect = addr_gate.port("wl_en")?.largest_rect(m2)?;
        let expanded_all = router.expand_to_grid(wl_en_rect, ExpandToGridStrategy::All);
        let src = wl_en_rect.with_hspan(Span::new(expanded_all.left(), wl_en_rect.left()));
        let src = router.expand_to_grid(src, ExpandToGridStrategy::Corner(Corner::LowerLeft));
        ctx.draw_rect(m2, src);
        router.occupy(m2, src, "wl_en")?;
        router.block(m2, expanded_all);

        let dst = control.port("wl_en")?.largest_rect(m1)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Side(Side::Top));
        ctx.draw_rect(m1, dst);

        to_route.push((m2, src, m1, dst, Some("wl_en")));

        let write_driver_en_rect = wmux_driver.port("wl_en")?.largest_rect(m2)?;
        let expanded_all = router.expand_to_grid(write_driver_en_rect, ExpandToGridStrategy::All);
        let src = write_driver_en_rect
            .with_hspan(Span::new(expanded_all.left(), write_driver_en_rect.left()));
        let src = router.expand_to_grid(src, ExpandToGridStrategy::Corner(Corner::LowerLeft));
        ctx.draw_rect(m2, src);
        router.occupy(m2, src, "write_driver_en")?;
        router.block(m2, expanded_all);

        let dst = control.port("write_driver_en")?.largest_rect(m1)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Side(Side::Top));
        ctx.draw_rect(m1, dst);

        to_route.push((m2, src, m1, dst, Some("write_driver_en")));

        let dummy_bl_rect = cols.port("dummy_bl")?.largest_rect(m1)?;
        let src_point = Point::new(cols.brect().left(), dummy_bl_rect.bottom() - 500);
        let src = router.expand_to_grid(Rect::from_point(src_point), ExpandToGridStrategy::Minimum);
        ctx.draw_rect(m1, src);
        ctx.draw(
            ElbowJog::builder()
                .src(dummy_bl_rect.edge(Side::Bot))
                .width2(320)
                .dst(src.center())
                .layer(m1)
                .build()
                .unwrap(),
        )?;

        let dst = control.port("dummy_bl")?.largest_rect(m1)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Corner(Corner::UpperLeft));
        ctx.draw_rect(m1, dst);

        to_route.push((m1, src, m1, dst, Some("dummy_bl")));

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

        let mut straps = RoutedStraps::new();
        straps.set_strap_layers([m2, m3]);

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

        // Connect replica bitcell wordline to control logic.
        let wl_en0_rect = rbl
            .port(PortId::new("wl", rbl_wl_index))?
            .first_rect(m2, Side::Right)?;
        let initial_right = wl_en0_rect.right();

        let wl_en0_rect = wl_en0_rect.expand_side(Side::Right, 1_000);
        let expanded_all = router.expand_to_grid(wl_en0_rect, ExpandToGridStrategy::All);
        let src = wl_en0_rect.with_hspan(Span::new(initial_right, expanded_all.right()));
        let src = src.expand_side(
            if rbl_wl_index % 2 == 0 {
                Side::Top
            } else {
                Side::Bot
            },
            420,
        );
        let via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m2, m3)
                .geometry(src, src)
                .build(),
        )?;
        let m3_src = router.expand_to_grid(src, ExpandToGridStrategy::Minimum);
        ctx.draw(via)?;
        ctx.draw_rect(m2, src);
        router.occupy(m2, src, "wl_en0")?;
        ctx.draw_rect(m3, m3_src);
        router.occupy(m3, m3_src, "wl_en0")?;

        let dst = control.port("wl_en0")?.largest_rect(m1)?;
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Side(Side::Top));
        ctx.draw_rect(m1, dst);

        to_route.push((m3, m3_src, m1, dst, Some("wl_en0")));

        for (lsrc, src, ldst, dst, net) in to_route {
            if let Some(net) = net {
                router.route_with_net(ctx, lsrc, src, ldst, dst, net)?;
            } else {
                router.route(ctx, lsrc, src, ldst, dst)?;
            }
        }

        router.block(m2, cols.brect());
        router.block(m3, cols.brect());

        // Connect column circuitry to power straps.
        for (dir, layer, expand) in [(Dir::Vert, m3, 3_800)] {
            for port_name in ["vdd", "vss"] {
                for port in cols.port(port_name)?.shapes(layer) {
                    if let Shape::Rect(rect) = port {
                        let rect = rect.with_span(rect.span(dir).expand_all(expand), dir);
                        router.block(layer, rect);
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
                        ctx.draw_rect(layer, rect);
                    }
                }
            }
        }

        // Connect decoders and DFFs to power straps.
        for (inst, layer) in [
            (&decoder, m1),
            (&addr_gate, m2),
            (&p1, m2),
            (&p2, m2),
            (&col_dec, m2),
            (&wmux_driver, m2),
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

        // Route column peripheral outputs to pins on bounding box of SRAM
        let groups = self.params.cols / self.params.mux_ratio;
        for (port, width) in [
            ("dout", groups),
            ("din", groups),
            ("wmask", self.params.wmask_width),
        ] {
            for i in 0..width {
                let port_id = PortId::new(port, i);
                let rect = cols.port(port_id.clone())?.largest_rect(m3)?;
                let rect = rect.with_vspan(rect.vspan().add_point(router_bbox.bottom()));
                ctx.draw_rect(m3, rect);
                ctx.add_port(CellPort::builder().id(port_id).add(m3, rect).build())?;
                router.block(m3, rect);
            }
        }

        let straps = straps.fill(&router, ctx)?;
        ctx.set_metadata(straps);

        ctx.draw(router)?;
        Ok(())
    }
}
