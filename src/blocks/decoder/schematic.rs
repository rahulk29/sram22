use itertools::Itertools;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use substrate::schematic::signal::Slice;

use super::{
    Decoder, DecoderParams, DecoderStage, DecoderStageParams, DecoderStagePhysicalDesignScript,
};
use crate::blocks::decoder::{base_indices, DecoderStagePhysicalDesign, RoutingStyle};
use crate::blocks::gate::{Gate, GateParams};

impl Decoder {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
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
            use_multi_finger_invs: true,
            dont_connect_outputs: true,
            child_sizes,
        };
        let mut inst = ctx
            .instantiate::<DecoderStage>(&params)?
            .with_connections([("vdd", vdd), ("vss", vss)]);
        ctx.bubble_filter_map(&mut inst, |port| {
            port.name().starts_with("y").then_some(port.name().into())
        });
        if node.children.is_empty() {
            ctx.bubble_filter_map(&mut inst, |port| {
                port.name()
                    .starts_with("predecode")
                    .then_some(port.name().into())
            });
        }

        let mut next_addr = (0, 0);
        for (i, node) in node.children.iter().enumerate() {
            let mut child = ctx
                .instantiate::<Decoder>(&DecoderParams {
                    pd: self.params.pd,
                    max_width: self
                        .params
                        .max_width
                        .map(|width| width / num_children as i64),
                    tree: super::DecoderTree { root: node.clone() },
                })?
                .with_connections([("vdd", vdd), ("vss", vss)]);

            let ports = child.ports()?.collect_vec();
            for child_port in ports
                .into_iter()
                .filter_map(|port| {
                    if port.name().starts_with("predecode") {
                        Some(port)
                    } else {
                        None
                    }
                })
                .sorted_unstable_by(|a, b| a.name().cmp(b.name()))
            {
                let port = ctx.port(
                    format!("predecode_{}_{}", next_addr.0, next_addr.1),
                    Direction::Input,
                );
                child.connect(child_port.name().clone(), port);
                if next_addr.1 > 0 {
                    next_addr = (next_addr.0 + 1, 0);
                } else {
                    next_addr = (next_addr.0, 1);
                }
            }

            let conn = ctx.bus(format!("child_conn_{i}"), node.num);
            let noconn = ctx.bus(format!("child_noconn_{i}"), node.num);

            child.connect("y", conn);
            if child.port("y_b").is_ok() {
                child.connect("y_b", noconn);
            }
            for j in 0..node.num {
                inst.connect(format!("predecode_{i}_{j}"), conn.index(j));
            }
            ctx.add_instance(child);
        }
        ctx.add_instance(inst);

        Ok(())
    }
}

impl DecoderStage {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let DecoderStagePhysicalDesign {
            gate_params,
            folding_factors,
            ..
        } = &*ctx
            .inner()
            .run_script::<DecoderStagePhysicalDesignScript>(&self.params)?;
        let num_stages = gate_params.len();
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let y = ctx.bus_port("y", self.params.num, Direction::Output);
        let y_b = if num_stages > 1 || gate_params[0].gate_type().is_and() {
            Some(ctx.bus_port("y_b", self.params.num, Direction::Output))
        } else {
            None
        };

        enum DecoderIO {
            Decoder { predecode: Vec<Vec<Slice>> },
            Driver { wl_en: Slice, inn: Slice },
        }
        let io = match self.params.routing_style {
            RoutingStyle::Decoder => {
                let mut predecode = Vec::new();
                for (i, s) in self.params.child_sizes.iter().copied().enumerate() {
                    predecode.push(Vec::new());
                    for j in 0..s {
                        predecode
                            .last_mut()
                            .unwrap()
                            .push(ctx.port(arcstr::format!("predecode_{i}_{j}"), Direction::Input));
                    }
                }
                DecoderIO::Decoder { predecode }
            }
            RoutingStyle::Driver => DecoderIO::Driver {
                wl_en: ctx.port("wl_en", Direction::Input),
                inn: ctx.bus_port("in", self.params.num, Direction::Input),
            },
        };
        let x: Vec<_> = (0..num_stages - 1)
            .map(|i| ctx.bus(format!("x_{i}"), self.params.num))
            .collect();

        let ports = ["a", "b", "c", "d"];
        for (stage, (gate, &folding_factor)) in
            gate_params.iter().zip(folding_factors.iter()).enumerate()
        {
            let gate_params = gate.scale(1. / (folding_factor as f64));

            for i in 0..self.params.num {
                for j in 0..folding_factor {
                    let mut gate = ctx
                        .instantiate::<Gate>(&gate_params)?
                        .with_connections([("vdd", vdd), ("vss", vss)])
                        .named(format!("gate_{}_{}_{}", stage, i, j));

                    if num_stages > 1 {
                        if stage == num_stages - 2 {
                            gate.connect("y", y_b.unwrap().index(i));
                        } else if stage == num_stages - 1 {
                            gate.connect("y", y.index(i));
                        } else if stage < num_stages - 1 {
                            gate.connect("y", x[stage].index(i));
                        }
                        if gate_params.gate_type().is_and() {
                            gate.connect("yb", ctx.signal(format!("y_b_noconn_{stage}_{i}_{j}")));
                        }
                    } else {
                        if gate_params.gate_type().is_and() {
                            gate.connect("yb", y_b.unwrap().index(i));
                        }
                        gate.connect("y", y.index(i));
                    }
                    if stage == 0 {
                        match &io {
                            DecoderIO::Decoder { predecode } => {
                                let idxs = base_indices(i, &self.params.child_sizes);
                                for (i, j) in idxs.into_iter().enumerate() {
                                    gate.connect(ports[i], predecode[i][j]);
                                }
                            }
                            DecoderIO::Driver { wl_en, inn } => {
                                gate.connect(ports[0], wl_en);
                                gate.connect(ports[1], inn.index(i));
                            }
                        }
                    } else if stage == num_stages - 1 {
                        gate.connect("a", y_b.unwrap().index(i));
                    } else {
                        gate.connect("a", x[stage - 1].index(i));
                    }
                    gate.add_to(ctx);
                }
            }
        }
        Ok(())
    }
}
