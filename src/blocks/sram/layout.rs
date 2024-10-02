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
use substrate::layout::routing::manual::jog::{ElbowJog, SJog};
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
        let col_params = self.col_params();
        let bitcells = ctx.instantiate::<SpCellArray>(&SpCellArrayParams {
            rows: self.params.rows(),
            cols: self.params.cols(),
            mux_ratio: self.params.mux_ratio(),
        })?;
        let mut cols = ctx.instantiate::<ColPeripherals>(&col_params)?;
        // TODO
        let tree = DecoderTree::new(self.params.row_bits(), 128e-15);
        // let decoder_params = DecoderStageParams {
        //     max_width: None,
        //     gate: tree.root.gate,
        //     invs: vec![],
        //     num: tree.root.num,
        //     child_sizes: tree.root.children.iter().map(|n| n.num).collect(),
        // };
        let mut decoder = ctx
            .instantiate::<Predecoder>(&DecoderParams { tree })?
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
            .with_orientation(Named::R90Cw);

        let col_tree = DecoderTree::new(self.params.col_select_bits(), 128e-15); // TODO
        let col_decoder_params = DecoderParams {
            tree: col_tree.clone(),
        };

        let mut col_dec = ctx
            .instantiate::<Predecoder>(&col_decoder_params)?
            .with_orientation(Named::R90Cw);
        let mut control = ctx.instantiate::<ControlLogicReplicaV2>(&NoParams)?;

        // TODO: decide how registers should be organized
        let num_dffs = self.params.addr_width() + 1;
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
        col_dec.align_beneath(bitcells.bbox(), 4_000);
        col_dec.align_to_the_left_of(cols.bbox(), 4_000);
        addr_gate.align_to_the_left_of(decoder.bbox(), 4_000);
        addr_gate.align_bottom(decoder.bbox());
        control.set_orientation(Named::FlipYx);
        control.align_beneath(decoder.bbox(), 4_000);
        control.align_to_the_left_of(control.bbox(), 4_000);
        replica_pc.align_beneath(rbl.bbox(), 4_000);
        replica_pc.align_centers_horizontally_gridded(rbl.bbox(), ctx.pdk().layout_grid());
        rbl.align_to_the_left_of(control.bbox(), 6_000);
        rbl.align_top(control.bbox());
        dffs.align_top(control.bbox());
        dffs.align_to_the_left_of(rbl.bbox(), 6_000);

        ctx.draw_ref(&bitcells)?;
        ctx.draw_ref(&cols)?;
        ctx.draw_ref(&decoder)?;
        ctx.draw_ref(&addr_gate)?;
        ctx.draw_ref(&col_dec)?;
        ctx.draw_ref(&control)?;
        ctx.draw_ref(&dffs)?;
        ctx.draw_ref(&rbl)?;
        ctx.draw_ref(&replica_pc)?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

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
            &dffs,
            &control,
            &cols,
            &replica_pc,
        ] {
            for layer in [m1, m2] {
                for shape in inst.shapes_on(layer) {
                    let rect = shape.brect();
                    router.block(layer, rect);
                }
            }
        }

        for inst in [&bitcells, &rbl] {
            router.block(m1, inst.brect().expand_dir(Dir::Vert, 6_000));
            router.block(m2, inst.brect().expand_dir(Dir::Horiz, 6_000));
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

        router.block(m2, cols.brect());

        // Connect column circuitry to power straps.
        for (dir, layer, expand) in [(Dir::Horiz, m2, 3_800)] {
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
