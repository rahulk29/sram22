use std::collections::HashMap;

use fanout::FanoutAnalyzer;
use serde::{Deserialize, Serialize};

use crate::clog2;
use crate::config::decoder::{Decoder24Params, DecoderParams};
use crate::config::gate::{GateParams, Size};
use crate::layout::decoder::get_idxs;
use crate::schematic::gate::{inv, nand2, nand3, Gate, GateType};
use crate::schematic::vlsir_api::{bus, concat, local_reference, signal, Instance, Module, Signal};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DecoderTree {
    pub root: TreeNode,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TreeNode {
    pub gate: Gate,
    pub buf: Option<Gate>,
    pub num: usize,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct PlanTreeNode {
    gate: GateType,
    buf: Option<GateType>,
    num: usize,
    children: Vec<PlanTreeNode>,
}

impl DecoderTree {
    pub fn new(bits: usize) -> Self {
        let plan = plan_decoder(bits, true);
        let root = size_decoder(&plan);
        DecoderTree { root }
    }
}

fn size_decoder(tree: &PlanTreeNode) -> TreeNode {
    let mut f = FanoutAnalyzer::new();

    let mut nodes = vec![];
    let mut curr = Some(tree);
    while let Some(node) = curr {
        nodes.push(node);
        curr = node.children.get(0);
    }
    nodes.reverse();

    for (i, node) in nodes.iter().enumerate() {
        f.add_gate(node.gate.into());
        if let Some(buf) = node.buf {
            f.add_gate(buf.into());
        }
        if let Some(next) = nodes.get(i + 1) {
            f.add_branch((next.num / node.num) as f64);
        }
    }
    // TODO use fanout results
    let res = f.size(32f64);
    let mut sizes = res.sizes().collect::<Vec<_>>();

    sizes.reverse();

    size_helper_tmp(tree, &sizes)
}

fn size_helper_tmp(x: &PlanTreeNode, _sizes: &[f64]) -> TreeNode {
    // TODO size decoder
    let buf = x.buf.map(|b| {
        Gate::new(
            b,
            Size {
                nmos_width: 1_600,
                pmos_width: 2_400,
            },
        )
    });

    TreeNode {
        gate: Gate::new(
            x.gate,
            Size {
                nmos_width: 3_200,
                pmos_width: 2_400,
            },
        ),
        buf,
        num: x.num,
        children: x
            .children
            .iter()
            .map(|n| size_helper_tmp(n, _sizes))
            .collect::<Vec<_>>(),
    }
}

fn plan_decoder(bits: usize, top: bool) -> PlanTreeNode {
    assert!(bits > 1);
    if bits == 2 {
        PlanTreeNode {
            gate: GateType::Nand2,
            buf: Some(GateType::Inv),
            num: 4,
            children: vec![],
        }
    } else if bits == 3 {
        PlanTreeNode {
            gate: GateType::Nand3,
            buf: Some(GateType::Inv),
            num: 8,
            children: vec![],
        }
    } else {
        let split = partition_bits(bits, top);
        let gate = match split.len() {
            2 => GateType::Nand2,
            3 => GateType::Nand3,
            _ => panic!("unexpected bit split"),
        };

        let children = split
            .into_iter()
            .map(|x| plan_decoder(x, false))
            .collect::<Vec<_>>();
        PlanTreeNode {
            gate,
            buf: Some(GateType::Inv),
            num: 2usize.pow(bits as u32),
            children,
        }
    }
}

fn partition_bits(bits: usize, top: bool) -> Vec<usize> {
    assert!(bits > 3);

    if top {
        let left = bits / 2;
        return vec![left, bits - left];
    }

    if bits % 2 == 0 {
        vec![bits / 2, bits / 2]
    } else if bits / 3 >= 2 {
        match bits % 3 {
            0 => vec![bits / 3, bits / 3, bits / 3],
            1 => vec![bits / 3 + 1, bits / 3, bits / 3],
            2 => vec![bits / 3 + 1, bits / 3 + 1, bits / 3],
            _ => panic!("unexpected remainder of `bits` divided by 3"),
        }
    } else {
        let left = bits / 2;
        vec![left, bits - left]
    }
}

pub fn hierarchical_decoder(params: &DecoderParams) -> Vec<Module> {
    let out = params.tree.root.num;
    let in_bits = clog2(out);

    let mut m = Module::new(&params.name);

    let vdd = signal("vdd");
    let gnd = signal("gnd");
    let addr = bus("addr", in_bits);
    let addr_b = bus("addr_b", in_bits);

    m.add_ports_inout(&[&vdd, &gnd]);
    m.add_ports_input(&[&addr, &addr_b]);
    m.add_ports_output(&[&bus("decode", out), &bus("decode_b", out)]);

    let mut gen = DecoderGen::new(params, &vdd, &gnd, &addr, &addr_b, in_bits);
    gen.helper(Some(&params.tree.root), 0);

    m.add_instances(gen.instances);
    gen.modules.push(m);

    gen.modules
}

struct DecoderGen<'a> {
    ctr: usize,
    params: &'a DecoderParams,
    vdd: &'a Signal,
    gnd: &'a Signal,
    addr: &'a Signal,
    addr_b: &'a Signal,
    modules: Vec<Module>,
    instances: Vec<Instance>,
    addr_bits: usize,
    nands: HashMap<(usize, Size), String>,
    invs: HashMap<Size, String>,
}

impl<'a> DecoderGen<'a> {
    pub fn new(
        params: &'a DecoderParams,
        vdd: &'a Signal,
        gnd: &'a Signal,
        addr: &'a Signal,
        addr_b: &'a Signal,
        addr_bits: usize,
    ) -> Self {
        Self {
            ctr: 0,
            params,
            vdd,
            gnd,
            addr,
            addr_b,
            modules: vec![],
            instances: vec![],
            addr_bits,
            nands: HashMap::new(),
            invs: HashMap::new(),
        }
    }

    fn get_id(&mut self) -> usize {
        self.ctr += 1;
        self.ctr
    }

    fn helper(&mut self, node: Option<&TreeNode>, depth: usize) -> Signal {
        if node.is_none() {
            assert!(self.addr_bits >= 1);
            self.addr_bits -= 1;
            return concat(vec![
                self.addr_b.get(self.addr_bits),
                self.addr.get(self.addr_bits),
            ]);
        }

        let node = node.unwrap();
        let gate_size = match node.gate.gate_type {
            GateType::Nand2 => 2,
            GateType::Nand3 => 3,
            _ => unreachable!(),
        };
        let sigs = (0..gate_size)
            .map(|i| node.children.get(i))
            .map(|n| self.helper(n, depth + 1))
            .collect::<Vec<_>>();
        let child_sizes = (0..gate_size)
            .map(|i| node.children.get(i).map(|n| n.num).unwrap_or(2))
            .collect::<Vec<_>>();

        assert_eq!(sigs.iter().map(|s| s.width()).product::<usize>(), node.num);

        let out_name = if depth == 0 {
            "decode".to_string()
        } else {
            format!("predecode_{}", self.get_id())
        };

        let out = bus(out_name, node.num);

        let nand_name = if let Some(nand_name) = self.nands.get(&(gate_size, node.gate.size)) {
            nand_name.to_string()
        } else {
            let nand_name = format!("{}_nand_{}", &self.params.name, self.get_id());
            let nand = match gate_size {
                2 => nand2(&GateParams {
                    name: nand_name.clone(),
                    size: node.gate.size,
                    length: self.params.lch,
                }),
                3 => nand3(&GateParams {
                    name: nand_name.clone(),
                    size: node.gate.size,
                    length: self.params.lch,
                }),
                _ => unreachable!(),
            };
            self.modules.push(nand);
            self.nands
                .insert((gate_size, node.gate.size), nand_name.clone());
            nand_name
        };

        let inv_name = if let Some(inv_name) = self.invs.get(&node.buf.unwrap().size) {
            inv_name.to_string()
        } else {
            let inv_name = format!("{}_inv_{}", &self.params.name, self.get_id());
            let inv = inv(&GateParams {
                name: inv_name.clone(),
                size: node.buf.unwrap().size,
                length: self.params.lch,
            });
            self.modules.push(inv);
            self.invs.insert(node.buf.unwrap().size, inv_name.clone());
            inv_name
        };

        for i in 0..node.num {
            let idxs = get_idxs(i, &child_sizes);

            let tmp = if depth != 0 {
                signal(format!("net_{}", self.get_id()))
            } else {
                // FIXME this should reference a connection slice, not a new signal
                signal(format!("decode_b[{i}]"))
            };

            assert!(node.children.len() <= 4);
            let ports = ["a", "b", "c", "d"].into_iter().take(gate_size);

            let mut nand = Instance::new(
                format!("nand_{}", self.get_id()),
                local_reference(nand_name.clone()),
            );
            nand.add_conns(&[("vdd", self.vdd), ("gnd", self.gnd), ("y", &tmp)]);

            for (j, port) in ports.enumerate() {
                nand.add_conn(port, &sigs[j].get(idxs[j]));
            }

            self.instances.push(nand);

            let mut inv =
                Instance::new(format!("inv_{}", self.get_id()), local_reference(inv_name.clone()));
            inv.add_conns(&[
                ("vdd", self.vdd),
                ("gnd", self.gnd),
                ("din", &tmp),
                ("din_b", &out.get(i)),
            ]);

            self.instances.push(inv);
        }

        out
    }
}

pub fn decoder_24(params: &Decoder24Params) -> Vec<Module> {
    let nand_name = format!("{}_nand", &params.name);
    let nand = nand2(&GateParams {
        name: nand_name.clone(),
        size: params.gate_size,
        length: params.lch,
    });

    let inv_name = format!("{}_inv", &params.name);
    let inv = inv(&GateParams {
        name: inv_name.clone(),
        size: params.inv_size,
        length: params.lch,
    });

    let vdd = signal("vdd");
    let gnd = signal("gnd");
    let din = bus("din", 2);
    let din_b = bus("din_b", 2);
    let dout = bus("dout", 4);

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &gnd]);
    m.add_ports_input(&[&din, &din_b]);
    m.add_port_output(&dout);

    for i in 0..4 {
        let tmp = signal(format!("out_b_{}", i));

        let mut nand = Instance::new(format!("nand_{}", i), local_reference(&nand_name));
        nand.add_conns(&[
            ("vdd", &vdd),
            ("gnd", &gnd),
            ("y", &tmp),
            ("a", &(if i % 2 == 0 { din.get(0) } else { din_b.get(0) })),
            (
                "b",
                &(if (i >> 1) % 2 == 0 {
                    din.get(1)
                } else {
                    din_b.get(1)
                }),
            ),
        ]);
        m.add_instance(nand);

        let mut inv = Instance::new(format!("inv_{}", i), local_reference(&inv_name));
        inv.add_conns(&[
            ("vdd", &vdd),
            ("gnd", &gnd),
            ("din", &tmp),
            ("dout", &dout.get(i)),
        ]);
        m.add_instance(inv);
    }

    vec![nand, inv, m]
}
