use std::collections::HashMap;

use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Dir, Rect, Shape, Side, Sign, Span};
use substrate::component::NoParams;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Instance, Port, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::routing::auto::grid::{
    ExpandToGridStrategy, JogToGrid, OffGridBusTranslation,
};
use substrate::layout::routing::auto::straps::{RoutedStraps, Target};
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::SJog;
use substrate::layout::routing::tracks::TrackLocator;
use substrate::layout::straps::SingleSupplyNet;
use substrate::layout::Draw;

use crate::v2::bitcell_array::replica::{ReplicaCellArray, ReplicaCellArrayParams};
use crate::v2::bitcell_array::{SpCellArray, SpCellArrayParams};
use crate::v2::columns::ColPeripherals;
use crate::v2::control::{ControlLogicReplicaV2, DffArray};
use crate::v2::decoder::layout::LastBitDecoderStage;
use crate::v2::decoder::{
    DecoderParams, DecoderStageParams, DecoderTree, Predecoder, WlDriver, WmuxDriver,
};

use super::SramInner;

impl SramInner {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let bitcells = ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows,
            cols: self.params.cols,
            mux_ratio: self.params.mux_ratio,
        })?;
        let mut cols = ctx.instantiate::<ColPeripherals>(&self.col_params())?;
        let tree = DecoderTree::new(self.params.row_bits);
        let decoder_params = DecoderStageParams {
            gate: tree.root.gate,
            num: tree.root.num,
            child_sizes: tree.root.children.iter().map(|n| n.num).collect(),
        };
        let mut decoder = ctx
            .instantiate::<LastBitDecoderStage>(&decoder_params)?
            .with_orientation(Named::R90Cw);
        let mut wl_driver = ctx
            .instantiate::<WlDriver>(&decoder_params)?
            .with_orientation(Named::R90Cw);

        let mut p1 = ctx.instantiate::<Predecoder>(&DecoderParams {
            tree: DecoderTree {
                root: tree.root.children[0].clone(),
            },
        })?;
        let p1_bits = tree.root.children[0].num.ilog2() as usize;
        let p2_bits = tree.root.children[1].num.ilog2() as usize;

        let mut p2 = ctx.instantiate::<Predecoder>(&DecoderParams {
            tree: DecoderTree {
                root: tree.root.children[1].clone(),
            },
        })?;

        let col_tree = DecoderTree::new(self.params.col_select_bits);
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };
        let col_bits = self.params.col_select_bits;

        let mut col_dec = ctx.instantiate::<Predecoder>(&col_decoder_params)?;
        let wmux_driver_params = DecoderStageParams {
            gate: col_tree.root.gate,
            num: col_tree.root.num,
            child_sizes: vec![],
        };
        let mut wmux_driver = ctx.instantiate::<WmuxDriver>(&wmux_driver_params)?;
        let _control = ctx.instantiate::<ControlLogicReplicaV2>(&NoParams)?;

        let num_dffs = self.params.addr_width + 1;
        let mut dffs = ctx.instantiate::<DffArray>(&num_dffs)?;

        let rbl_rows = ((self.params.rows / 12) + 1) * 2;
        let mut rbl = ctx.instantiate::<ReplicaCellArray>(&ReplicaCellArrayParams {
            rows: rbl_rows,
            cols: 2,
        })?;

        cols.align_beneath(bitcells.bbox(), 4_310);
        cols.align_centers_horizontally_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        wl_driver.align_to_the_left_of(bitcells.bbox(), 6_350);
        wl_driver.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_to_the_left_of(wl_driver.bbox(), 1_270);
        decoder.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        decoder.align_side_to_grid(Side::Left, 680);
        rbl.align_to_the_left_of(decoder.bbox(), 4_310);
        rbl.align_centers_vertically_gridded(bitcells.bbox(), ctx.pdk().layout_grid());
        rbl.align_side_to_grid(Side::Left, 680);
        p1.align_beneath(wl_driver.bbox(), 1_270);
        p1.align_right(wl_driver.bbox());
        p2.align_beneath(p1.bbox(), 1_270);
        p2.align_right(wl_driver.bbox());
        wmux_driver.align_beneath(p2.bbox(), 1_270);
        wmux_driver.align_right(wl_driver.bbox());
        col_dec.align_beneath(wmux_driver.bbox(), 1_270);
        col_dec.align_right(wl_driver.bbox());
        // control.align_beneath(col_dec.bbox(), 1_270);
        // control.align_right(wl_driver.bbox());
        dffs.align_beneath(col_dec.bbox(), 3_000);
        dffs.align_right(wl_driver.bbox());

        ctx.draw_ref(&bitcells)?;
        ctx.draw_ref(&cols)?;
        ctx.draw_ref(&decoder)?;
        ctx.draw_ref(&wl_driver)?;
        ctx.draw_ref(&wmux_driver)?;
        ctx.draw_ref(&p1)?;
        ctx.draw_ref(&p2)?;
        ctx.draw_ref(&col_dec)?;
        // ctx.draw_ref(&control)?;
        ctx.draw_ref(&dffs)?;
        ctx.draw_ref(&rbl)?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        let router_bbox = ctx
            .brect()
            .expand(4 * 680)
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

        router.block(m2, cols.brect());
        router.block(m3, cols.brect());

        // TODO: add back &control
        for inst in [&p1, &p2, &col_dec, &wmux_driver, &dffs] {
            for shape in inst.shapes_on(m2) {
                let rect = shape.brect();
                router.block(m2, rect);
            }
            for shape in inst.shapes_on(m1) {
                let rect = shape.brect();
                router.block(m1, rect);
            }
        }

        for inst in [&bitcells, &rbl] {
            router.block(m2, inst.brect().expand_dir(Dir::Horiz, 6_000));
            router.block(m3, inst.brect());
        }

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
                        PortId::new("addr", i)
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

        // Route address bits from DFFs to decoders
        let mut ctr = 0;
        for (inst, num) in [(&p1, p1_bits), (&p2, p2_bits), (&col_dec, col_bits)] {
            let bottom_port = inst
                .port(&format!("predecode_{}_1", num - 1))?
                .largest_rect(m2)?;
            let on_grid_bus = router.register_off_grid_bus_translation(
                OffGridBusTranslation::builder()
                    .layer(m2)
                    .line_and_space(320, 160)
                    .output(bottom_port.edge(Side::Left))
                    .start(bottom_port.side(Side::Bot))
                    .n(2 * num as i64)
                    .build(),
            );

            let mut ports: Vec<Rect> = on_grid_bus.ports().collect();
            ports.reverse();

            for i in 0..num {
                let src = dffs.port(PortId::new("q", ctr))?.largest_rect(m2)?;
                let src = router.expand_to_layer_grid(src, m2, ExpandToGridStrategy::Minimum);
                let src = router.expand_to_layer_grid(src, m3, ExpandToGridStrategy::Minimum);
                ctx.draw_rect(m2, src);
                let dst = ports[2 * i];
                router.route(ctx, m2, src, m2, dst)?;
                let src = dffs.port(PortId::new("qn", ctr))?.largest_rect(m2)?;
                let src = router.expand_to_layer_grid(src, m2, ExpandToGridStrategy::Minimum);
                let src = router.expand_to_layer_grid(src, m3, ExpandToGridStrategy::Minimum);
                ctx.draw_rect(m2, src);
                let dst = ports[2 * i + 1];
                router.route(ctx, m2, src, m2, dst)?;
                ctr += 1;
            }
        }

        let left_port = decoder
            .port(&format!("predecode_0_{}", tree.root.children[0].num - 1))?
            .largest_rect(m1)?;
        let p0_bus = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .layer(m1)
                .line_and_space(320, 160)
                .output(left_port.edge(Side::Bot))
                .start(left_port.side(Side::Left))
                .n(tree.root.children[0].num as i64)
                .build(),
        );
        let mut p0_ports = p0_bus.ports().collect::<Vec<Rect>>();
        p0_ports.reverse();

        let left_port = decoder
            .port(&format!("predecode_1_{}", tree.root.children[1].num - 1))?
            .largest_rect(m1)?;
        let p1_bus = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .layer(m1)
                .line_and_space(320, 160)
                .output(left_port.edge(Side::Bot))
                .start(left_port.side(Side::Left))
                .n(tree.root.children[1].num as i64)
                .build(),
        );
        let mut p1_ports = p1_bus.ports().collect::<Vec<Rect>>();
        p1_ports.reverse();

        // Route predecoders to final decoder stage
        for (i, &dst) in p0_ports.iter().enumerate().take(tree.root.children[0].num) {
            let src = p1.port(PortId::new("decode", i))?.largest_rect(m0)?;
            let src = router.register_jog_to_grid(
                JogToGrid::builder()
                    .layer(m0)
                    .rect(src)
                    .dst_layer(m1)
                    .width(170)
                    .first_dir(Side::Bot)
                    .second_dir(if i % 2 == 0 { Side::Right } else { Side::Left })
                    .build(),
            );
            router.route(ctx, m1, src, m1, dst)?;
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
                    .first_dir(Side::Bot)
                    .second_dir(if i % 2 == 0 { Side::Right } else { Side::Left })
                    .build(),
            );
            router.route(ctx, m1, src, m1, dst)?;
            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(src, src)
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        // Route wordline decoder to wordline driver
        for i in 0..tree.root.num {
            let src = decoder.port(PortId::new("decode", i))?.largest_rect(m0)?;
            let dst = wl_driver.port(PortId::new("in", i))?.largest_rect(m0)?;
            let jog = SJog::builder()
                .src(src)
                .dst(dst)
                .dir(Dir::Horiz)
                .layer(m0)
                .width(170)
                .grid(ctx.pdk().layout_grid())
                .build()
                .unwrap();
            ctx.draw(jog)?;
        }

        // Route wordline driver to bitcell array
        for i in 0..tree.root.num {
            let src = wl_driver.port(PortId::new("decode", i))?.largest_rect(m0)?;
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
                    let merged_spans = Span::merge_adjacent(spans, |a, b| a.min_distance(b) < 200);

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
        for port_name in ["wl_dummy", "bl_dummy", "br_dummy"] {
            for i in 0..2 {
                port_ids.push((
                    PortId::new(port_name, i),
                    match port_name {
                        "wl_dummy" => SingleSupplyNet::Vss,
                        "bl_dummy" | "br_dummy" => SingleSupplyNet::Vdd,
                        _ => unreachable!(),
                    },
                ));
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
        for i in 0..rbl_rows {
            if i != rbl_rows / 2 {
                port_ids.push((PortId::new("wl", i), SingleSupplyNet::Vss));
            }
        }
        connect_bitcells_to_straps(&rbl, port_ids)?;

        // Connect column circuitry to power straps.
        for (dir, layer, expand) in [(Dir::Vert, m3, 3_800), (Dir::Horiz, m2, 3_800)] {
            for port_name in ["vdd", "vss"] {
                for port in cols.port(port_name)?.shapes(layer) {
                    if let Shape::Rect(rect) = port {
                        let rect = rect.with_span(rect.span(dir).expand_all(expand), dir);
                        router.block(m3, rect);
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
            (&wl_driver, m1),
            (&p1, m2),
            (&p2, m2),
            (&col_dec, m2),
            (&wmux_driver, m2),
            (&dffs, m2),
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
