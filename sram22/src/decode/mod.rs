use itertools::iproduct;
use micro_hdl::{context::Context, node::Node};

use crate::{
    cells::gates::{inv::Inv, nand::Nand2Gate, GateSize},
    clog2,
};

/// The root of the `HierarchicalDecoder` is the predecoder
#[micro_hdl::module]
struct HierarchicalDecoder {
    #[params]
    tree: NandDecoder,
    #[input]
    addr: Vec<Node>,
    #[input]
    addr_b: Vec<Node>,
    #[output]
    decode: Vec<Node>,
    #[inout]
    gnd: Node,
    #[inout]
    vdd: Node,
}

#[derive(Debug, Clone)]
pub struct NandDecoder {
    /// The number of bits output by this level of decoder.
    /// Equals the product of the `output_bits` of all children.
    output_bits: usize,
    /// The number of inputs to the nand gate
    gate_inputs: usize,
    /// The number of inputs to each gate in a decoder is the number of children
    children: Vec<NandDecoder>,
}

impl NandDecoder {
    pub fn is_valid(&self) -> bool {
        self.output_bits % 2 == 0
            && (self.children.is_empty()
                || (self
                    .children
                    .iter()
                    .map(|c| c.output_bits)
                    .product::<usize>()
                    == self.output_bits
                    && self.children.len() == self.gate_inputs
                    && self.children.iter().all(|c| c.is_valid())))
    }
}

impl HierarchicalDecoder {
    fn generate(tree: NandDecoder, c: &mut Context) -> HierarchicalDecoderInstance {
        let decode = c.bus(tree.output_bits);
        let gnd = c.node();
        let vdd = c.node();

        let (addr, addr_b) = Self::generate_inner(&tree, c, decode.clone(), gnd, vdd);
        println!("generated {} addr bits", addr.len());

        Self::instance()
            .tree(tree)
            .addr(addr)
            .addr_b(addr_b)
            .decode(decode)
            .gnd(gnd)
            .vdd(vdd)
            .build()
    }

    fn generate_inner(
        tree: &NandDecoder,
        c: &mut Context,
        out: Vec<Node>,
        gnd: Node,
        vdd: Node,
    ) -> (Vec<Node>, Vec<Node>) {
        if tree.children.is_empty() {
            assert_eq!(tree.gate_inputs, 2);
            let addr_bits = clog2(tree.output_bits) as usize;
            let addr = c.bus(addr_bits);
            let addr_b = c.bus(addr_bits);
            // TODO bug here
            for (ctr, (&a, &b)) in iproduct!(&addr, &addr_b).enumerate() {
                let gate = Nand2Gate::instance()
                    .size(GateSize::minimum())
                    .a(a)
                    .b(b)
                    .y(out[ctr])
                    .gnd(gnd)
                    .vdd(vdd)
                    .build();
                c.add(gate);
            }
            (addr, addr_b)
        } else {
            assert_eq!(tree.children.len(), 2);
            let in1 = c.bus(tree.children[0].output_bits);
            let in2 = c.bus(tree.children[1].output_bits);

            for (ctr, (&i1, &i2)) in iproduct!(&in1, &in2).enumerate() {
                let tmp = c.node();
                let gate = Nand2Gate::instance()
                    .size(GateSize::minimum())
                    .a(i1)
                    .b(i2)
                    .y(tmp)
                    .gnd(gnd)
                    .vdd(vdd)
                    .build();
                c.add(gate);

                let inv = Inv::instance()
                    .size(GateSize::minimum())
                    .din(tmp)
                    .dout(out[ctr])
                    .gnd(gnd)
                    .vdd(vdd)
                    .build();
                c.add(inv);
            }

            let (mut a1, mut a1b) = Self::generate_inner(&tree.children[0], c, in1, gnd, vdd);
            let (mut a2, mut a2b) = Self::generate_inner(&tree.children[1], c, in2, gnd, vdd);

            a1.append(&mut a2);
            a1b.append(&mut a2b);

            (a1, a1b)
        }
    }

    fn name(tree: NandDecoder) -> String {
        format!("hier_decode_{}", tree.output_bits)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use super::*;
    use micro_hdl::{backend::spice::SpiceBackend, frontend::parse};

    #[test]
    fn valid_16row_decoder() {
        let decoder = NandDecoder {
            output_bits: 16,
            gate_inputs: 2,
            children: vec![
                NandDecoder {
                    output_bits: 4,
                    gate_inputs: 2,
                    children: vec![],
                },
                NandDecoder {
                    output_bits: 4,
                    gate_inputs: 2,
                    children: vec![],
                },
            ],
        };

        assert!(decoder.is_valid());
    }

    #[test]
    fn netlist_16row_decoder() -> Result<(), Box<dyn std::error::Error>> {
        let out = <Vec<u8>>::new();
        let _b = SpiceBackend::new(out);

        let decoder = NandDecoder {
            output_bits: 16,
            gate_inputs: 2,
            children: vec![
                NandDecoder {
                    output_bits: 4,
                    gate_inputs: 2,
                    children: vec![],
                },
                NandDecoder {
                    output_bits: 4,
                    gate_inputs: 2,
                    children: vec![],
                },
            ],
        };
        assert!(decoder.is_valid());

        let tree = parse(HierarchicalDecoder::top(decoder));
        let file = tempfile::tempfile()?;
        let mut backend = SpiceBackend::with_file(file)?;
        backend.netlist(&tree)?;
        let mut file = backend.output();

        let mut s = String::new();
        file.seek(SeekFrom::Start(0))?;
        file.read_to_string(&mut s)?;
        println!("{}", &s);

        Ok(())
    }
}
