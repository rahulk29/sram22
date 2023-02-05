use std::collections::HashSet;

use grid::Grid;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::error::Result as SubResult;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Element, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::transform::Translate;
use substrate::layout::geom::{Corner, Point, Rect, Sign, Span};
use substrate::layout::group::elements::ElementGroup;

use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;

use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::tracks::UniformTracks;
use substrate::script::Script;

use crate::v2::gate::{Gate, GateParams};

use super::{DecoderParams, DecoderStage, DecoderStageParams, Predecoder};

pub struct LastBitDecoderStage {
    params: DecoderStageParams,
}

impl Component for LastBitDecoderStage {
    type Params = DecoderStageParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("last_bit_decoder_stage")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<LastBitDecoderPhysicalDesignScript>(&NoParams)?;
        let mut gate = ctx.instantiate::<DecoderGate>(&DecoderGateParams {
            gate: self.params.gate,
            dsn: (*dsn).clone(),
        })?;
        gate.set_orientation(Named::R90Cw);
        let col = (0..self.params.num)
            .map(|_| gate.clone().into())
            .collect::<Vec<_>>();
        let mut grid = Grid::new(0, 0);
        grid.push_col(col);
        let tiler = GridTiler::new(grid);
        ctx.draw_ref(&tiler)?;
        let tracks = UniformTracks::builder()
            .line(dsn.line)
            .space(dsn.space)
            .start(ctx.brect().left())
            .sign(Sign::Neg)
            .build()
            .unwrap();
        let vspan = ctx.brect().vspan();
        let mut child_tracks = Vec::new();
        let mut idx = 0usize;
        for (i, s) in self.params.child_sizes.iter().copied().enumerate() {
            child_tracks.push(Vec::new());
            for j in 0..s {
                let tr = tracks.index(idx);
                let rect = Rect::from_spans(tr, vspan);
                ctx.draw_rect(dsn.vm, rect);
                ctx.add_port(CellPort::with_shape(
                    arcstr::format!("decode_{i}_{j}"),
                    dsn.vm,
                    rect,
                ));
                idx += 1;
                child_tracks[i].push(rect);
            }
        }
        for n in 0..self.params.num {
            let idxs = base_indices(n, &self.params.child_sizes);
            let tf = tiler.translation(n, 0);
            let mut gate = gate.clone();
            gate.translate(tf);
            let ports = ["a", "b", "c", "d"];
            for (i, j) in idxs.into_iter().enumerate() {
                // connect to child_tracks[i][j].
                let port = gate.port(ports[i])?.largest_rect(dsn.li)?;
                let track = child_tracks[i][j];

                let bot = Rect::from_spans(track.hspan(), port.vspan());
                let viap = ViaParams::builder()
                    .layers(dsn.li, dsn.vm)
                    .geometry(bot, bot)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw_ref(&via)?;

                ctx.draw_rect(
                    dsn.li,
                    Rect::from_spans(port.hspan().union(via.brect().hspan()), port.vspan()),
                )
            }
            ctx.add_port(
                gate.port("y")?
                    .into_cell_port()
                    .named(arcstr::format!("decode_{n}")),
            );
        }
        Ok(())
    }
}

impl Predecoder {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> SubResult<()> {
        let dsn = ctx
            .inner()
            .run_script::<PredecoderPhysicalDesignScript>(&NoParams)?;
        let node = &self.params.tree.root;
        let child_sizes = if node.children.is_empty() {
            (0..node.num.ilog2()).map(|_| 2).collect()
        } else {
            node.children.iter().map(|n| n.num).collect()
        };
        let params = DecoderStageParams {
            gate: node.gate,
            num: node.num,
            child_sizes,
        };
        let mut inst = ctx.instantiate::<DecoderStage>(&params)?;
        inst.place(Corner::LowerLeft, Point::zero());
        ctx.add_ports(inst.ports_starting_with("decode"));

        let mut x = 0;
        for (i, node) in node.children.iter().enumerate() {
            let mut child = ctx.instantiate::<Predecoder>(&DecoderParams {
                tree: super::DecoderTree { root: node.clone() },
            })?;
            child.place(Corner::UpperLeft, Point::new(x, 0));
            x += child.brect().width() + 2 * dsn.width;

            for j in 0..node.num {
                let src = child.port(&format!("decode_{j}"))?.largest_rect(dsn.li)?;
                let dst = inst
                    .port(&format!("predecode_{i}_{j}"))?
                    .largest_rect(dsn.hm)?;
                let rect = Rect::from_spans(src.hspan(), dst.vspan().add_point(src.top()));
                ctx.draw_rect(dsn.vm, rect);
            }
            ctx.draw(child)?;
        }
        ctx.draw(inst)?;

        ctx.flatten();

        Ok(())
    }
}

enum PredecoderTracks {
    /// Forward through any higher predecoders.
    Forward,
    /// Backward through any lower predecoders.
    Backward,
    /// Hop from one stage to the next.
    Hop,
    /// Power rails (ground if odd, power if even).
    Power,
}

fn base_indices(mut i: usize, sizes: &[usize]) -> Vec<usize> {
    let mut res = Vec::new();
    for sz in sizes {
        res.push(i % sz);
        i /= sz;
    }
    res
}

impl DecoderStage {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<PredecoderPhysicalDesignScript>(&NoParams)?;
        let gate = ctx.instantiate::<DecoderGate>(&DecoderGateParams {
            gate: self.params.gate,
            dsn: (*dsn).clone(),
        })?;
        let row = (0..self.params.num)
            .map(|_| gate.clone().into())
            .collect::<Vec<_>>();
        let mut grid = Grid::new(0, 0);
        grid.push_row(row);
        let tiler = GridTiler::new(grid);
        ctx.draw_ref(&tiler)?;
        let tracks = UniformTracks::builder()
            .line(dsn.line)
            .space(dsn.space)
            .start(ctx.brect().bottom())
            .sign(Sign::Neg)
            .build()
            .unwrap();
        let hspan = ctx.brect().hspan();
        let mut child_tracks = Vec::new();
        let mut idx = 0usize;
        for (i, s) in self.params.child_sizes.iter().copied().enumerate() {
            child_tracks.push(Vec::new());
            for j in 0..s {
                let tr = tracks.index(idx);
                let rect = Rect::from_spans(hspan, tr);
                ctx.draw_rect(dsn.hm, rect);
                ctx.add_port(CellPort::with_shape(
                    arcstr::format!("predecode_{i}_{j}"),
                    dsn.hm,
                    rect,
                ));
                idx += 1;
                child_tracks[i].push(rect);
            }
        }
        for n in 0..self.params.num {
            let idxs = base_indices(n, &self.params.child_sizes);
            let tf = tiler.translation(0, n);
            let mut gate = gate.clone();
            gate.translate(tf);
            let ports = ["a", "b", "c", "d"];
            for (i, j) in idxs.into_iter().enumerate() {
                // connect to child_tracks[i][j].
                let port = gate.port(ports[i])?.largest_rect(dsn.li)?;
                let track = child_tracks[i][j];

                let bot = Rect::from_spans(port.hspan(), track.vspan());
                let viap = ViaParams::builder()
                    .layers(dsn.li, dsn.vm)
                    .geometry(bot, bot)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw_ref(&via)?;
                let viap = ViaParams::builder()
                    .layers(dsn.vm, dsn.hm)
                    .geometry(via.brect(), track)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw_ref(&via)?;

                ctx.draw_rect(
                    dsn.li,
                    Rect::from_spans(port.hspan(), port.vspan().union(via.brect().vspan())),
                )
            }
            ctx.add_port(
                gate.port("y")?
                    .into_cell_port()
                    .named(arcstr::format!("decode_{n}")),
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DecoderGateParams {
    pub gate: GateParams,
    pub dsn: PhysicalDesign,
}

pub struct DecoderGate {
    pub params: DecoderGateParams,
}

impl Component for DecoderGate {
    type Params = DecoderGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("decoder_gate")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = &self.params.dsn;

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let hspan = Span::until(dsn.width);
        let mut gate = ctx.instantiate::<Gate>(&self.params.gate)?;
        gate.set_orientation(Named::R90);
        gate.place_center_x(dsn.width / 2);
        ctx.add_ports(gate.ports());
        ctx.draw(gate)?;

        ctx.flatten();

        let mut group = ElementGroup::new();
        for elem in ctx.elems() {
            if dsn.abut_layers.contains(&elem.layer.layer()) {
                let rect = Rect::from_spans(hspan, elem.brect().vspan());
                group.add(Element::new(elem.layer.clone(), rect));
            }
        }

        ctx.draw(group)?;

        ctx.draw_rect(outline, Rect::from_spans(hspan, ctx.brect().vspan()));
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PhysicalDesign {
    /// Width of a decoder cell.
    width: i64,
    /// Width of a decoder tap cell.
    tap_width: i64,
    /// The horizontal metal layer above (`vm`)[PhysicalDesign::vm].
    hm: LayerKey,
    /// The vertical metal layer above (`li`)[PhysicalDesign::li].
    vm: LayerKey,
    /// The metal used to connect to MOS sources, drains, gates, and taps.
    li: LayerKey,
    /// Width of wires in bus.
    line: i64,
    /// Spacing between wires in bus.
    space: i64,
    /// Width of power rail.
    rail_width: i64,
    /// Layers that should be extended to the edge of decoder gates and tap cells.
    abut_layers: HashSet<LayerKey>,
}

pub struct PredecoderPhysicalDesignScript;

impl Script for PredecoderPhysicalDesignScript {
    type Params = NoParams;
    type Output = PhysicalDesign;

    fn run(
        _params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let layers = ctx.layers();
        let li = layers.get(Selector::Metal(0))?;
        let vm = layers.get(Selector::Metal(1))?;
        let hm = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 2_920,
            tap_width: 1_300,
            hm,
            vm,
            li,
            line: 320,
            space: 160,
            rail_width: 180,
            abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
        })
    }
}

pub struct LastBitDecoderPhysicalDesignScript;

impl Script for LastBitDecoderPhysicalDesignScript {
    type Params = NoParams;
    type Output = PhysicalDesign;

    fn run(
        _params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let layers = ctx.layers();
        let li = layers.get(Selector::Metal(0))?;
        let vm = layers.get(Selector::Metal(1))?;
        let hm = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 1_580,
            tap_width: 790,
            hm,
            vm,
            li,
            line: 320,
            space: 160,
            rail_width: 180,
            abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
        })
    }
}
