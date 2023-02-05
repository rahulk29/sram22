use std::collections::{HashMap, HashSet};
use std::iter::Extend;

use grid::{grid, Grid};
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::layout::cell::Instance;
use substrate::layout::cell::{CellPort, Element, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::transform::Translate;
use substrate::layout::geom::{Rect, Sign, Span};
use substrate::layout::group::elements::ElementGroup;
use substrate::layout::group::Group;
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;
use substrate::layout::placement::grid::{ArrayTiler, GridTiler};
use substrate::layout::placement::nine_patch::{NpTiler, Region};
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::tracks::UniformTracks;
use substrate::layout::Draw;
use substrate::script::Script;

use crate::v2::gate::{Gate, GateParams};

use super::{Decoder, DecoderStage, DecoderStageParams, DecoderTree};

pub struct LastBitDecoderStage {
    params: DecoderStageParams,
}

fn decoder_stage_layout(
    ctx: &mut LayoutCtx,
    params: &DecoderStageParams,
    dsn: &PhysicalDesign,
) -> Result<()> {
    // TODO: Parameter validation
    let decoder_params = DecoderGateParams {
        gate: params.gate.clone(),
        dsn: (*dsn).clone(),
    };
    let gate = ctx.instantiate::<DecoderGate>(&decoder_params)?;
    let tap = ctx.instantiate::<DecoderTap>(&decoder_params)?;

    let mut tiler = ArrayTiler::new();

    for _ in 0..params.num / dsn.tap_period / 2 {
        tiler.push_num(gate.clone(), dsn.tap_period);
        tiler.push(tap.clone());
        tiler.push_num(gate.clone(), dsn.tap_period);
    }

    let grid = tiler.into_grid_tiler();

    ctx.draw_ref(&grid)?;

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
    for (i, s) in params.child_sizes.iter().copied().enumerate() {
        child_tracks.push(Vec::new());
        for j in 0..s {
            let tr = tracks.index(idx);
            let rect = Rect::from_spans(hspan, tr);
            ctx.draw_rect(dsn.routing_metal, rect);
            ctx.add_port(CellPort::with_shape(
                arcstr::format!("predecode_{i}_{j}"),
                dsn.routing_metal,
                rect,
            ));
            idx += 1;
            child_tracks[i].push(rect);
        }
    }
    for n in 0..params.num {
        let idxs = base_indices(n, &params.child_sizes);
        let n_calc = n + (n + dsn.tap_period) / dsn.tap_period / 2;
        println!("{}", n_calc);
        let tf = grid.translation(0, n_calc);
        let mut gate = gate.clone();
        gate.translate(tf);
        let ports = ["a", "b", "c", "d"];
        for (i, j) in idxs.into_iter().enumerate() {
            // connect to child_tracks[i][j].
            let port = gate.port(ports[i])?.largest_rect(dsn.li)?;
            let track = child_tracks[i][j];

            let bot = Rect::from_spans(port.hspan(), track.vspan());

            let mut via_metals = Vec::new();
            via_metals.push(dsn.li);
            via_metals.extend(dsn.via_metals.clone());
            via_metals.push(dsn.routing_metal);

            let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                rect: bot,
                via_metals,
            })?;

            ctx.draw_ref(&via)?;

            ctx.draw_rect(
                dsn.li,
                Rect::from_spans(port.hspan(), port.vspan().union(via.brect().vspan())),
            );
        }
        ctx.add_port(
            gate.port("y")?
                .into_cell_port()
                .named(arcstr::format!("decode_{n}")),
        );
    }
    Ok(())
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
        decoder_stage_layout(ctx, &self.params, &dsn)
    }
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
        decoder_stage_layout(ctx, &self.params, &dsn)
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

/// Relevant spans to be exported to the cell metadata of `DecoderGate`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DecoderGateSpans {
    /// Span of layers that need to be abutted between adjacent cells.
    abutted_layers: HashMap<LayerKey, Vec<Span>>,
    /// Mapping of routing span to span of enclosing diffusion layer.
    met_to_diff: HashMap<Span, Span>,
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
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;

        let hspan = Span::until(dsn.width);
        let mut gate = ctx.instantiate::<Gate>(&self.params.gate)?;
        gate.set_orientation(Named::R90);
        gate.place_center_x(dsn.width / 2);
        ctx.add_ports(gate.ports());
        ctx.draw(gate)?;

        ctx.flatten();

        let mut abutted_layers = HashMap::new();
        let mut met_to_diff = HashMap::new();

        let mut group = ElementGroup::new();
        for elem in ctx.elems() {
            if dsn.abut_layers.contains(&elem.layer.layer()) {
                let rect = Rect::from_spans(hspan, elem.brect().vspan());
                group.add(Element::new(elem.layer.clone(), rect));
                abutted_layers
                    .entry(elem.layer.layer())
                    .or_insert(Vec::new())
                    .push(elem.brect().vspan());
            }
        }

        let rects: Vec<Rect> = abutted_layers[&nsdm]
            .iter()
            .chain(abutted_layers[&psdm].iter())
            .map(|span| {
                let vspan = Span::from_center_span_gridded(
                    span.center(),
                    dsn.rail_width,
                    ctx.pdk().layout_grid(),
                );
                met_to_diff.insert(vspan, *span);
                Rect::from_spans(hspan, vspan)
            })
            .collect();

        for rect in rects {
            ctx.draw_rect(dsn.routing_metal, rect);
            abutted_layers
                .entry(dsn.routing_metal)
                .or_insert(Vec::new())
                .push(rect.vspan());
        }

        ctx.draw(group)?;

        ctx.draw_rect(outline, Rect::from_spans(hspan, ctx.brect().vspan()));

        abutted_layers
            .entry(outline)
            .or_insert(Vec::new())
            .push(ctx.brect().vspan());
        ctx.set_metadata::<DecoderGateSpans>(DecoderGateSpans {
            abutted_layers,
            met_to_diff,
        });

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DecoderTap {
    params: DecoderGateParams,
}

impl Component for DecoderTap {
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
        arcstr::literal!("decoder_tap")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = &self.params.dsn;

        let layers = ctx.layers();
        let tap = layers.get(Selector::Name("tap"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;

        let hspan = Span::until(dsn.tap_width);
        let decoder_gate = ctx.instantiate::<DecoderGate>(&self.params)?;

        let gate_spans = decoder_gate.cell().get_metadata::<DecoderGateSpans>();

        for (mut layer, spans) in gate_spans.abutted_layers.iter() {
            // P+ tap for NMOS, N+ tap for PMOS
            if *layer == nsdm {
                layer = &psdm;
            } else if *layer == psdm {
                layer = &nsdm;
            }
            for vspan in spans {
                ctx.draw_rect(*layer, Rect::from_spans(hspan, *vspan));
            }
        }

        let mut via_metals = Vec::new();
        via_metals.push(dsn.li);
        via_metals.extend(dsn.via_metals.clone());
        via_metals.push(dsn.routing_metal);
        if let Some(spans) = gate_spans.abutted_layers.get(&dsn.routing_metal) {
            for vspan in spans {
                let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                    rect: Rect::from_spans(hspan, gate_spans.met_to_diff[vspan]),
                    via_metals: vec![tap, dsn.li],
                })?;
                ctx.draw(via)?;
                let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                    rect: Rect::from_spans(hspan, *vspan),
                    via_metals: via_metals.clone(),
                })?;
                ctx.draw(via)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DecoderViaParams {
    rect: Rect,
    via_metals: Vec<LayerKey>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecoderVia {
    params: DecoderViaParams,
}

impl Component for DecoderVia {
    type Params = DecoderViaParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        if params.via_metals.len() < 2 {
            return Err(substrate::component::error::Error::InvalidParams.into());
        }
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("decoder_via")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let rect = self.params.rect;
        let mut via_metals = self.params.via_metals.iter();

        let mut prev_layer = *via_metals.next().unwrap();
        for metal in via_metals {
            let viap = ViaParams::builder()
                .layers(prev_layer, *metal)
                .geometry(rect, rect)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;

            prev_layer = *metal;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PhysicalDesign {
    /// Width of a decoder cell.
    width: i64,
    /// Width of a decoder tap cell.
    tap_width: i64,
    /// Number of decoders on either side of each tap.
    tap_period: usize,
    /// The metal layer used for routing and power rails.
    routing_metal: LayerKey,
    /// List of intermediate layers in via between (`li`)[PhysicalDesign::li] and
    /// (`routing_metal`)[PhysicalDesign::routing_metal)
    via_metals: Vec<LayerKey>,
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
        let routing_metal = layers.get(Selector::Metal(2))?;
        let via_metals = vec![layers.get(Selector::Metal(1))?];
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 5_840,
            tap_width: 1_300,
            tap_period: 1,
            routing_metal,
            via_metals,
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
        let routing_metal = layers.get(Selector::Metal(1))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 1_580,
            tap_width: 790,
            tap_period: 4,
            routing_metal,
            via_metals: vec![],
            li,
            line: 320,
            space: 160,
            rail_width: 320,
            abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
        })
    }
}
