use std::collections::{HashMap, HashSet};
use std::iter::Extend;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::error::Result;
use substrate::index::IndexOwned;

use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Corner, Dir, Point, Rect, Sign, Span};
use substrate::layout::cell::{CellPort, Element, Port, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::group::elements::ElementGroup;
use substrate::layout::DrawRef;

use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;

use substrate::layout::placement::align::AlignMode;
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::OffsetJog;
use substrate::layout::routing::tracks::UniformTracks;
use substrate::script::Script;

use crate::blocks::gate::{Gate, GateParams};

use super::{DecoderParams, DecoderStage, DecoderStageParams, Predecoder};

pub struct LastBitDecoderStage {
    params: DecoderStageParams,
}

pub enum RoutingStyle {
    Decoder,
    Driver,
}

pub(crate) fn decoder_stage_layout(
    ctx: &mut LayoutCtx,
    params: &DecoderStageParams,
    dsn: &PhysicalDesign,
    routing_style: RoutingStyle,
) -> Result<()> {
    // TODO: Parameter validation
    let decoder_params = DecoderGateParams {
        gate: params.gate,
        dsn: (*dsn).clone(),
    };
    let gate = ctx.instantiate::<DecoderGate>(&decoder_params)?;
    let mut flipped_gate = ctx.instantiate::<DecoderGate>(&decoder_params)?;
    flipped_gate.set_orientation(Named::ReflectHoriz);
    let tap = ctx.instantiate::<DecoderTap>(&decoder_params)?;

    let mut period_tiler = ArrayTiler::builder();

    for _ in 0..dsn.tap_period / 2 {
        period_tiler.push(flipped_gate.clone()).push(gate.clone());
    }

    let mut period_tiler = period_tiler
        .push(tap.clone())
        .mode(AlignMode::ToTheRight)
        .alt_mode(AlignMode::CenterVertical)
        .build();

    period_tiler.expose_ports(
        |port: CellPort, i| match port.id().name().as_ref() {
            "vdd" | "vss" => Some(port),
            "y" => Some(port.named("decode").with_index(i)),
            "y_b" => Some(port.named("decode_b").with_index(i)),
            _ => Some(port.with_index(i)),
        },
        PortConflictStrategy::Merge,
    )?;

    let period_group = period_tiler.draw_ref()?;

    let mut tiler = ArrayTiler::builder()
        .push(tap)
        .push_num(period_group, params.num / dsn.tap_period)
        .mode(AlignMode::ToTheRight)
        .alt_mode(AlignMode::CenterVertical)
        .build();

    tiler.expose_ports(
        |port: CellPort, i| {
            let index = if i > 0 { dsn.tap_period * (i - 1) } else { 0 } + port.id().index();
            match port.name().as_ref() {
                "vdd" | "vss" => Some(port),
                "decode" => Some(port.with_index(index)),
                "decode_b" => Some(port.with_index(index)),
                _ => Some(port.with_index(index)),
            }
        },
        PortConflictStrategy::Merge,
    )?;
    ctx.add_ports(
        tiler
            .ports()
            .cloned()
            .filter_map(|port| match port.name().as_str() {
                "vdd" | "vss" | "decode" | "decode_b" => Some(port),
                "b" => Some(port.named("in")),
                _ => None,
            }),
    )
    .unwrap();

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
    match routing_style {
        RoutingStyle::Decoder => {
            for (i, s) in params.child_sizes.iter().copied().enumerate() {
                child_tracks.push(Vec::new());
                for j in 0..s {
                    let tr = tracks.index(idx);
                    let rect = Rect::from_spans(hspan, tr);
                    ctx.draw_rect(dsn.stripe_metal, rect);
                    ctx.add_port(CellPort::with_shape(
                        arcstr::format!("predecode_{i}_{j}"),
                        dsn.stripe_metal,
                        rect,
                    ))
                    .unwrap();
                    idx += 1;
                    child_tracks[i].push(rect);
                }
            }
        }
        RoutingStyle::Driver => {
            let tr = tracks.index(0usize);
            let rect = Rect::from_spans(hspan, tr);
            child_tracks.push(vec![rect]);
            ctx.draw_rect(dsn.stripe_metal, rect);
            ctx.add_port(CellPort::with_shape(
                arcstr::literal!("wl_en"),
                dsn.stripe_metal,
                rect,
            ))
            .unwrap();
        }
    }

    let mut via_metals = Vec::new();
    via_metals.push(dsn.li);
    via_metals.extend(dsn.via_metals.clone());
    via_metals.push(dsn.stripe_metal);
    let ports = ["a", "b", "c", "d"];

    for n in 0..params.num {
        match routing_style {
            RoutingStyle::Decoder => {
                let idxs = base_indices(n, &params.child_sizes);
                for (i, j) in idxs.into_iter().enumerate() {
                    // connect to child_tracks[i][j].
                    let port = tiler
                        .port_map()
                        .port(PortId::new(ports[i], n))?
                        .largest_rect(dsn.li)?;
                    let track = child_tracks[i][j];

                    let bot = Rect::from_spans(port.hspan(), track.vspan());

                    let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                        rect: bot,
                        via_metals: via_metals.clone(),
                    })?;

                    ctx.draw_ref(&via)?;

                    ctx.draw_rect(
                        dsn.li,
                        Rect::from_spans(port.hspan(), port.vspan().union(via.brect().vspan())),
                    );
                }
            }
            RoutingStyle::Driver => {
                // connect to child_tracks[0][0].
                let port = tiler
                    .port_map()
                    .port(PortId::new(ports[0], n))?
                    .largest_rect(dsn.li)?;
                let track = child_tracks[0][0];

                let bot = Rect::from_spans(port.hspan(), track.vspan());

                let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                    rect: bot,
                    via_metals: via_metals.clone(),
                })?;

                ctx.draw_ref(&via)?;

                ctx.draw_rect(
                    dsn.li,
                    Rect::from_spans(port.hspan(), port.vspan().union(via.brect().vspan())),
                );
            }
        };
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
        decoder_stage_layout(ctx, &self.params, &dsn, RoutingStyle::Decoder)
    }
}

impl Predecoder {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
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
        ctx.add_ports(inst.ports_starting_with("decode")).unwrap();
        if node.children.is_empty() {
            ctx.add_ports(inst.ports_starting_with("predecode"))
                .unwrap();
        }
        ctx.add_port(inst.port("vdd")?).unwrap();
        ctx.add_port(inst.port("vss")?).unwrap();

        let mut x = 0;
        let mut next_addr = (0, 0);
        for (i, node) in node.children.iter().enumerate() {
            let mut child = ctx.instantiate::<Predecoder>(&DecoderParams {
                tree: super::DecoderTree { root: node.clone() },
            })?;
            child.place(Corner::UpperLeft, Point::new(x, 0));
            x += child.brect().width() + dsn.width * dsn.tap_period as i64;

            for port in child
                .ports_starting_with("predecode")
                .sorted_unstable_by(|a, b| a.name().cmp(b.name()))
            {
                ctx.add_port(port.named(format!("predecode_{}_{}", next_addr.0, next_addr.1)))
                    .unwrap();
                if next_addr.1 > 0 {
                    next_addr = (next_addr.0 + 1, 0);
                } else {
                    next_addr = (next_addr.0, 1);
                }
            }

            for j in 0..node.num {
                let src = child.port(PortId::new("decode", j))?.largest_rect(dsn.li)?;
                let dst = inst
                    .port(&format!("predecode_{i}_{j}"))?
                    .largest_rect(dsn.stripe_metal)?;
                let rect =
                    Rect::from_spans(src.hspan(), Span::new(src.top() - src.width(), src.top()));
                let jog = OffsetJog::builder()
                    .dir(Dir::Horiz)
                    .sign(if j % 2 == 0 { Sign::Pos } else { Sign::Neg })
                    .src(rect)
                    .space(335)
                    .dst(dst.top())
                    .layer(dsn.li)
                    .build()
                    .unwrap();

                let mut via_metals = Vec::new();
                via_metals.push(dsn.li);
                via_metals.extend(dsn.via_metals.clone());
                via_metals.push(dsn.stripe_metal);

                let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                    rect: jog.r2().bbox().intersection(dst.bbox()).into_rect(),
                    via_metals,
                })?;

                ctx.draw(jog)?;
                ctx.draw(via)?;
            }
            ctx.draw(child)?;
        }
        ctx.draw(inst)?;

        ctx.flatten();

        Ok(())
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
        decoder_stage_layout(ctx, &self.params, &dsn, RoutingStyle::Decoder)
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
    /// Mapping of routing span to the name of its corresponding port and the
    /// span of enclosing diffusion layer.
    met_to_diff: HashMap<Span, (String, Span)>,
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
        ctx.add_ports(gate.ports()).unwrap();
        ctx.draw_ref(&gate)?;

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

        let spans: Vec<(Span, &str)> = abutted_layers[&nsdm]
            .iter()
            .map(|span| (*span, "vss"))
            .chain(abutted_layers[&psdm].iter().map(|span| (*span, "vdd")))
            .collect();

        let mut via_metals = Vec::new();
        via_metals.push(dsn.li);
        via_metals.extend(dsn.via_metals.clone());
        via_metals.push(dsn.stripe_metal);

        for (span, port_name) in spans {
            let vspan = Span::from_center_span_gridded(
                span.center(),
                dsn.rail_width,
                ctx.pdk().layout_grid(),
            );
            met_to_diff.insert(vspan, (port_name.to_string(), span));
            let rect = Rect::from_spans(hspan, vspan);
            ctx.draw_rect(dsn.stripe_metal, rect);
            ctx.merge_port(CellPort::with_shape(port_name, dsn.stripe_metal, rect));
            abutted_layers
                .entry(dsn.stripe_metal)
                .or_insert(Vec::new())
                .push(rect.vspan());
            for port_rect in gate
                .port(port_name)?
                .shapes(dsn.li)
                .filter_map(|shape| shape.as_rect())
            {
                let intersection = rect.intersection(port_rect.bbox());
                if !intersection.is_empty() {
                    let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                        rect: intersection.into_rect(),
                        via_metals: via_metals.clone(),
                    })?;
                    ctx.draw(via)?;
                }
            }
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

        for (layer, spans) in gate_spans.abutted_layers.iter() {
            // P+ tap for NMOS, N+ tap for PMOS
            if *layer == nsdm || *layer == psdm {
                continue;
            }
            for vspan in spans {
                let rect = Rect::from_spans(hspan, *vspan);
                ctx.draw_rect(*layer, rect);
                if *layer == dsn.stripe_metal {
                    let (port_name, _) = &gate_spans.met_to_diff[vspan];
                    ctx.merge_port(CellPort::with_shape(port_name, *layer, rect));
                }
            }
        }

        let hspan = hspan.shrink_all(65);

        if let Some(spans) = gate_spans.abutted_layers.get(&nsdm) {
            for vspan in spans {
                ctx.draw_rect(psdm, Rect::from_spans(hspan, (*vspan).shrink_all(110)));
            }
        }

        if let Some(spans) = gate_spans.abutted_layers.get(&psdm) {
            for vspan in spans {
                ctx.draw_rect(nsdm, Rect::from_spans(hspan, (*vspan).shrink_all(110)));
            }
        }

        let hspan = hspan.shrink_all(125);

        let mut via_metals = Vec::new();
        via_metals.push(dsn.li);
        via_metals.extend(dsn.via_metals.clone());
        via_metals.push(dsn.stripe_metal);
        if let Some(spans) = gate_spans.abutted_layers.get(&dsn.stripe_metal) {
            for vspan in spans {
                let (_, vspan_diff) = gate_spans.met_to_diff[vspan];
                let via = ctx.instantiate::<DecoderVia>(&DecoderViaParams {
                    rect: Rect::from_spans(hspan, vspan_diff.shrink_all(290)),
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
    pub(crate) width: i64,
    /// Width of a decoder tap cell.
    pub(crate) tap_width: i64,
    /// Number of decoders on either side of each tap.
    pub(crate) tap_period: usize,
    /// The metal layer used for buses and power rails.
    pub(crate) stripe_metal: LayerKey,
    /// The metal layer used for connecting stripes to individual decoders.
    pub(crate) wire_metal: LayerKey,
    /// List of intermediate layers in via between (`li`)[PhysicalDesign::li] and
    /// (`stripe_metal`)[PhysicalDesign::stripe_metal)
    pub(crate) via_metals: Vec<LayerKey>,
    /// The metal used to connect to MOS sources, drains, gates, and taps.
    pub(crate) li: LayerKey,
    /// Width of wires in bus.
    pub(crate) line: i64,
    /// Spacing between wires in bus.
    pub(crate) space: i64,
    /// Width of power rail.
    pub(crate) rail_width: i64,
    /// Layers that should be extended to the edge of decoder gates and tap cells.
    pub(crate) abut_layers: HashSet<LayerKey>,
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
        let stripe_metal = layers.get(Selector::Metal(2))?;
        let wire_metal = layers.get(Selector::Metal(1))?;
        let via_metals = vec![layers.get(Selector::Metal(1))?];
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 2_000,
            tap_width: 790,
            tap_period: 2,
            stripe_metal,
            wire_metal,
            via_metals,
            li,
            line: 320,
            space: 160,
            rail_width: 320,
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
        let stripe_metal = layers.get(Selector::Metal(1))?;
        let wire_metal = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 1_580,
            tap_width: 1_580,
            tap_period: 8,
            stripe_metal,
            wire_metal,
            via_metals: vec![],
            li,
            line: 320,
            space: 160,
            rail_width: 320,
            abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
        })
    }
}
