use std::collections::VecDeque;

use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::signal::{Signal, Slice};

use substrate::index::IndexOwned;

use super::{Decoder, DecoderStage, DecoderStageParams, TreeNode};
use crate::clog2;
use crate::v2::decoder::get_idxs;
use crate::v2::gate::{And2, And3, GateParams, Inv, Nand2, Nand3};

impl Decoder {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let out_bits = self.params.tree.root.num;
        let mut in_bits = clog2(out_bits);

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let addr = ctx.bus_port("addr", in_bits, Direction::Input);
        let addr_b = ctx.bus_port("addr_b", in_bits, Direction::Input);
        let decode = ctx.bus_port("decode", out_bits, Direction::Output);
        let decode_b = ctx.bus_port("decode_b", out_bits, Direction::Output);

        let port_names = vec!["a", "b", "c"];

        // Initialize all gates in the decoder tree using BFS.
        let mut queue = VecDeque::<(Option<Slice>, &TreeNode)>::new();
        queue.push_back((None, &self.params.tree.root));
        let mut ctr = 0;

        while let Some((output_port, node)) = queue.pop_front() {
            ctr += 1;
            let gate_size = node.gate.num_inputs();
            let mut stage = ctx.instantiate::<DecoderStage>(&DecoderStageParams {
                gate: node.gate,
                num: node.num,
                child_sizes: (0..gate_size)
                    .map(|i| node.children.get(i).map(|child| child.num).unwrap_or(2))
                    .collect(),
            })?;
            stage.connect_all([("vdd", &vdd), ("vss", &vss)]);

            if let Some(output_port) = output_port {
                let unused_wire = ctx.bus(format!("unused_{ctr}"), output_port.width());
                stage.connect_all([("decode", &output_port), ("decode_b", &unused_wire)]);
            } else {
                println!("drive decode");
                stage.connect_all([("decode", &decode), ("decode_b", &decode_b)]);
            }

            for (i, &port_name) in port_names.iter().enumerate().take(gate_size) {
                let input_signal = if let Some(child) = node.children.get(i) {
                    let input_bus = ctx.bus(format!("{port_name}_{ctr}"), child.num);
                    queue.push_back((Some(input_bus), child));
                    input_bus.into()
                } else {
                    assert!(in_bits >= 1);
                    in_bits -= 1;
                    Signal::new(vec![addr_b.index(in_bits), addr.index(in_bits)])
                };
                stage.connect(port_name, input_signal);
            }
            ctx.add_instance(stage);
        }

        Ok(())
    }
}

impl DecoderStage {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let num = self.params.num;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let decode = ctx.bus_port("decode", num, Direction::Output);
        let decode_b = ctx.bus_port("decode_b", num, Direction::Output);

        let port_names = [
            arcstr::literal!("a"),
            arcstr::literal!("b"),
            arcstr::literal!("c"),
        ];

        // Instantiate NAND gate.
        println!("{:?}", self.params.gate);
        let (and, gate_size) = match self.params.gate {
            GateParams::And2(params) => (ctx.instantiate::<And2>(&params)?, 2),
            GateParams::And3(params) => (ctx.instantiate::<And3>(&params)?, 3),
            _ => unreachable!(),
        };

        assert_eq!(self.params.child_sizes.len(), gate_size);
        assert_eq!(self.params.child_sizes.iter().product::<usize>(), num);

        let input_ports = (0..gate_size)
            .map(|i| {
                ctx.bus_port(
                    port_names[i].clone(),
                    self.params.child_sizes[i],
                    Direction::Input,
                )
            })
            .collect::<Vec<Slice>>();

        for i in 0..num {
            let idxs = get_idxs(i, &self.params.child_sizes);

            let mut and = and.clone();
            and.connect_all([
                ("vdd", &vdd),
                ("vss", &vss),
                ("yb", &decode_b.index(i)),
                ("y", &decode.index(i)),
            ]);
            for j in 0..gate_size {
                and.connect(port_names[j].clone(), input_ports[j].index(idxs[j]));
            }
            ctx.add_instance(and);
        }

        Ok(())
    }
}
