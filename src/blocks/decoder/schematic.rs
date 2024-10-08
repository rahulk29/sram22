use itertools::Itertools;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use substrate::index::IndexOwned;

use super::layout::{decoder_stage_schematic, DecoderPhysicalDesignScript, RoutingStyle};
use super::{Decoder, DecoderParams, DecoderStage, DecoderStageParams};
use crate::blocks::gate::GateParams;
use crate::clog2;

impl Decoder {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let out_bits = self.params.tree.root.num;
        let in_bits = clog2(out_bits);

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let port_names = ["a", "b", "c"];
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
        let dsn = ctx
            .inner()
            .run_script::<DecoderPhysicalDesignScript>(&self.params.pd)?;
        decoder_stage_schematic(ctx, &self.params, &dsn, self.params.routing_style)
    }
}
