use std::collections::HashMap;
use std::iter::Extend;

use itertools::Itertools;
use serde::Serialize;
use substrate::component::Component;
use substrate::error::Result;
use substrate::index::IndexOwned;

use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Corner, Dir, Point, Rect, Side, Sign, Span};
use substrate::layout::cell::{CellPort, Element, Flatten, Port, PortConflictStrategy, PortId};
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
use crate::blocks::decoder::{base_indices, Decoder, DecoderParams, DecoderPhysicalDesign, DecoderPhysicalDesignScript, DecoderStage, DecoderStageParams, DecoderStagePhysicalDesign, DecoderStagePhysicalDesignScript, RoutingStyle};
use crate::blocks::gate::{Gate, GateParams};

struct Metadata {
    final_stage_width: i64,
}

impl Decoder {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<DecoderPhysicalDesignScript>(&self.params.pd)?;
        let mut node = &self.params.tree.root;
        let mut invs = vec![];

        let num_children = node.children.len();
        while num_children == 1 {
            if let GateParams::Inv(params) | GateParams::FoldedInv(params) = node.gate {
                invs.push(params);
                node = &node.children[0];
            } else {
                break;
            }
        }
        invs.reverse();
        let child_sizes = if node.children.is_empty() {
            (0..node.num.ilog2()).map(|_| 2).collect()
        } else {
            node.children.iter().map(|n| n.num).collect()
        };
        let params = DecoderStageParams {
            pd: self.params.pd,
            routing_style: RoutingStyle::Decoder,
            max_width: self.params.max_width,
            gate: node.gate,
            invs,
            num: node.num,
            child_sizes,
        };
        let stage_dsn = ctx
            .inner()
            .run_script::<DecoderStagePhysicalDesignScript>(&params)?;
        ctx.set_metadata(Metadata {
            final_stage_width: stage_dsn.dsn.width,
        });
        let mut inst = ctx.instantiate::<DecoderStage>(&params)?;
        inst.place(Corner::LowerRight, Point::zero());
        ctx.add_ports(inst.ports_starting_with("y")).unwrap();
        if node.children.is_empty() {
            ctx.add_ports(inst.ports_starting_with("predecode"))
                .unwrap();
        }
        ctx.add_port(inst.port("vdd")?).unwrap();
        ctx.add_port(inst.port("vss")?).unwrap();

        let mut x = 0;
        let mut next_addr = (0, 0);
        for (i, node) in node.children.iter().enumerate() {
            let mut child = ctx.instantiate::<Decoder>(&DecoderParams {
                pd: self.params.pd,
                max_width: self
                    .params
                    .max_width
                    .map(|width| width / num_children as i64),
                tree: super::DecoderTree { root: node.clone() },
            })?;
            child.place(Corner::UpperRight, Point::new(x, -340));
            x -= (child.brect().width() as usize)
                .div_ceil(dsn.width as usize * dsn.tap_period + dsn.tap_width as usize)
                as i64
                * (dsn.width * dsn.tap_period as i64 + dsn.tap_width);
            ctx.merge_port(child.port("vdd")?);
            ctx.merge_port(child.port("vss")?);

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

            let final_stage_width = child.cell().get_metadata::<Metadata>().final_stage_width;
            for j in 0..node.num {
                let src = child
                    .port(PortId::new("y", j))?
                    .largest_rect(dsn.li)
                    .unwrap();
                let src = src.expand_side(Side::Top, 340);
                let dst = inst
                    .port(format!("predecode_{i}_{j}"))?
                    .largest_rect(dsn.stripe_metal)
                    .unwrap();
                ctx.draw_rect(dsn.li, src);
                let rect =
                    Rect::from_spans(src.hspan(), Span::with_stop_and_length(src.top(), 170));
                let jog = OffsetJog::builder()
                    .dir(Dir::Horiz)
                    .sign(Sign::Neg)
                    .src(rect)
                    .space(final_stage_width / 2 - 170)
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

impl DecoderStage {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let DecoderStagePhysicalDesign {
            gate_params,
            max_folding_factor,
            folding_factors,
            dsn,
        } = &*ctx
            .inner()
            .run_script::<DecoderStagePhysicalDesignScript>(&self.params)?;
        let mut tiler = ArrayTiler::builder();
        let num_stages = gate_params.len();

        for (gate, &folding_factor) in gate_params.iter().zip(folding_factors.iter()) {
            let decoder_params = DecoderGateParams {
                gate: gate.scale(1. / (folding_factor as f64)),
                filler: false,
                dsn: dsn.clone(),
            };
            let gate = ctx.instantiate::<DecoderGate>(&decoder_params)?;
            let filler_gate = ctx.instantiate::<DecoderGate>(&DecoderGateParams {
                filler: true,
                ..decoder_params.clone()
            })?;
            let tap = ctx.instantiate::<DecoderTap>(&decoder_params)?;

            let mut stage_tiler = ArrayTiler::builder();

            stage_tiler.push(tap.clone());
            for i in 0..self.params.num {
                for j in 0..*max_folding_factor {
                    if j < folding_factor {
                        stage_tiler.push(gate.clone());
                    } else {
                        stage_tiler.push(filler_gate.clone());
                    }
                    if (max_folding_factor * i + j) % dsn.tap_period == dsn.tap_period - 1 {
                        stage_tiler.push(tap.clone());
                    }
                }
            }

            if (self.params.num * max_folding_factor) % dsn.tap_period != 0 {
                stage_tiler.push(tap.clone());
            }

            let mut stage_tiler = stage_tiler
                .mode(AlignMode::ToTheRight)
                .alt_mode(AlignMode::CenterVertical)
                .build();

            stage_tiler.expose_ports(
                |port: CellPort, i| {
                    let idx = if i > 0 {
                        i - (i / (dsn.tap_period + 1) + 1)
                    } else {
                        0
                    };
                    match port.id().name().as_ref() {
                        "vdd" | "vss" => Some(port),
                        _ => Some(port.with_index(idx)),
                    }
                },
                PortConflictStrategy::Merge,
            )?;

            let stage_group = stage_tiler.draw_ref()?;
            tiler.push(stage_group);
        }

        let mut tiler = tiler
            .mode(AlignMode::Above)
            .space(300)
            .alt_mode(AlignMode::CenterHorizontal)
            .build();

        tiler.expose_ports(
            |port: CellPort, i| {
                let idx = port.id().index();
                match port.name().as_ref() {
                    "vdd" | "vss" => Some(port),
                    _ => Some(port.with_index(idx * num_stages + i)),
                }
            },
            PortConflictStrategy::Merge,
        )?;

        for stage in 0..num_stages - 1 {
            let folding_factor = folding_factors[stage];
            for i in 0..self.params.num {
                let inv_in: Vec<_> = tiler
                    .port_map()
                    .port(PortId::new(
                        "a",
                        i * folding_factors[stage + 1] * num_stages + stage + 1,
                    ))?
                    .shapes(dsn.li)
                    .filter_map(|shape| shape.as_rect())
                    .collect();
                let gate_out = tiler
                    .port_map()
                    .port(PortId::new("y", i * folding_factor * num_stages + stage))?
                    .largest_rect(dsn.li)?;
                for j in 0..folding_factor {
                    let src = tiler
                        .port_map()
                        .port(PortId::new(
                            "y",
                            (i * folding_factor + j) * num_stages + stage,
                        ))?
                        .largest_rect(dsn.li)?;
                    for inv_in in &inv_in {
                        let jog = OffsetJog::builder()
                            .dir(subgeom::Dir::Vert)
                            .sign(subgeom::Sign::Pos)
                            .src(src)
                            .dst(inv_in.left())
                            .layer(dsn.li)
                            .space(170)
                            .build()
                            .unwrap();
                        let rect = Rect::from_spans(
                            inv_in.hspan(),
                            Span::new(jog.r2().bottom(), inv_in.top()),
                        );
                        ctx.draw(jog)?;
                        ctx.draw_rect(dsn.li, rect);
                    }
                }
                for j in 0..folding_factors[stage + 1] {
                    for dst in tiler
                        .port_map()
                        .port(PortId::new(
                            "a",
                            (i * folding_factors[stage + 1] + j) * num_stages + stage + 1,
                        ))?
                        .shapes(dsn.li)
                        .filter_map(|shape| shape.as_rect())
                    {
                        let jog = OffsetJog::builder()
                            .dir(subgeom::Dir::Vert)
                            .sign(subgeom::Sign::Pos)
                            .src(gate_out)
                            .dst(dst.left())
                            .layer(dsn.li)
                            .space(170)
                            .build()
                            .unwrap();
                        let rect =
                            Rect::from_spans(dst.hspan(), Span::new(jog.r2().bottom(), dst.top()));
                        ctx.draw(jog)?;
                        ctx.draw_rect(dsn.li, rect);
                    }
                }
            }
        }
        ctx.add_ports(
            tiler
                .ports()
                .cloned()
                .filter_map(|port| match port.name().as_str() {
                    "vdd" | "vss" => Some(port),
                    _ => None,
                }),
        )
        .unwrap();

        ctx.draw_ref(&tiler)?;

        // expose decoder outputs
        {
            let folding_factor = folding_factors[num_stages - 1];
            for n in 0..self.params.num {
                // connect folded outputs
                if folding_factor > 1 {
                    let left_port = tiler
                        .port_map()
                        .port(PortId::new(
                            "y",
                            n * folding_factor * num_stages + num_stages - 1,
                        ))?
                        .largest_rect(dsn.li)?;
                    for k in 0..folding_factor {
                        let port = tiler
                            .port_map()
                            .port(PortId::new(
                                "y",
                                (n * folding_factor + k) * num_stages + num_stages - 1,
                            ))?
                            .largest_rect(dsn.li)?;
                        let jog = OffsetJog::builder()
                            .dir(subgeom::Dir::Vert)
                            .sign(subgeom::Sign::Pos)
                            .src(port)
                            .dst(left_port.left())
                            .layer(dsn.li)
                            .space(170)
                            .build()
                            .unwrap();
                        ctx.draw(jog)?;
                    }
                }
                ctx.add_port(
                    tiler
                        .port_map()
                        .port(PortId::new(
                            "y",
                            n * folding_factor * num_stages + num_stages - 1,
                        ))?
                        .clone()
                        .with_id(PortId::new(arcstr::format!("y"), n)),
                )?;
                if num_stages > 1 {
                    ctx.add_port(
                        tiler
                            .port_map()
                            .port(PortId::new(
                                "y",
                                n * folding_factor * num_stages + num_stages - 2,
                            ))?
                            .clone()
                            .with_id(PortId::new(arcstr::format!("y_b"), n)),
                    )?;
                } else if let GateParams::And2(_) | GateParams::And3(_) = &gate_params[0] {
                    ctx.add_port(
                        tiler
                            .port_map()
                            .port(PortId::new("y_b", n * folding_factor * num_stages))?
                            .clone()
                            .with_id(PortId::new(arcstr::format!("y_b"), n)),
                    )?;
                }
            }
        }

        let folding_factor = folding_factors[0];
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
        match self.params.routing_style {
            RoutingStyle::Decoder => {
                for (i, s) in self.params.child_sizes.iter().copied().enumerate() {
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

        for n in 0..self.params.num {
            match self.params.routing_style {
                RoutingStyle::Decoder => {
                    let idxs = base_indices(n, &self.params.child_sizes);
                    for (i, j) in idxs.into_iter().enumerate() {
                        for k in 0..folding_factors[0] {
                            // connect to child_tracks[i][j].
                            let port = tiler
                                .port_map()
                                .port(PortId::new(
                                    ports[i],
                                    (n * folding_factors[0] + k) * num_stages,
                                ))?
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
                                Rect::from_spans(
                                    port.hspan(),
                                    port.vspan().union(via.brect().vspan()),
                                ),
                            );
                        }
                    }
                }
                RoutingStyle::Driver => {
                    for k in 0..folding_factor {
                        // connect to child_tracks[0][0].
                        let port = tiler
                            .port_map()
                            .port(PortId::new(ports[0], (n * folding_factor + k) * num_stages))?
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

                    // connect folded gates
                    if folding_factor > 1 {
                        let left_port = tiler
                            .port_map()
                            .port(PortId::new(ports[1], n * folding_factor * num_stages))?
                            .largest_rect(dsn.li)?;
                        for k in 0..folding_factor {
                            let port = tiler
                                .port_map()
                                .port(PortId::new(ports[1], (n * folding_factor + k) * num_stages))?
                                .largest_rect(dsn.li)?;
                            let jog = OffsetJog::builder()
                                .dir(subgeom::Dir::Vert)
                                .sign(subgeom::Sign::Neg)
                                .src(left_port)
                                .dst(port.right())
                                .layer(dsn.li)
                                .space(170)
                                .build()
                                .unwrap();
                            ctx.draw(jog)?;
                        }
                    }
                    ctx.add_port(
                        tiler
                            .port_map()
                            .port(PortId::new(ports[1], n * folding_factor * num_stages))?
                            .clone()
                            .with_id(PortId::new(arcstr::format!("in"), n)),
                    )?;
                }
            };
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DecoderGateParams {
    pub gate: GateParams,
    pub filler: bool,
    pub dsn: DecoderPhysicalDesign,
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
        if !self.params.filler {
            ctx.add_ports(gate.ports()).unwrap();
        }
        let mut gate_group = gate.draw_ref()?;

        gate_group.flatten();

        let mut abutted_layers = HashMap::new();
        let mut met_to_diff = HashMap::new();

        let mut group = ElementGroup::new();
        for elem in gate_group.elements() {
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
            if !self.params.filler {
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
        }

        let outline_vspan = gate_group.bbox().union(ctx.bbox()).into_rect().vspan();
        ctx.draw_rect(outline, Rect::from_spans(hspan, outline_vspan));
        if !self.params.filler {
            ctx.draw(gate_group)?;
        }

        ctx.draw(group)?;

        abutted_layers
            .entry(outline)
            .or_insert(Vec::new())
            .push(outline_vspan);
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

        let hspan = hspan.shrink_all(170);

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
