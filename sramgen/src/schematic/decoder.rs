use std::collections::HashMap;

use fanout::FanoutAnalyzer;

use crate::layout::decoder::get_idxs;
use crate::schematic::gate::{inv, nand2, nand3, Gate, GateParams, GateType, Size};
use crate::utils::conns::conn_slice;
use crate::utils::{conn_map, log2, sig_conn, signal, BusConnection};
use pdkprims::config::Int;
use serde::{Deserialize, Serialize};
use vlsir::circuit::connection::Stype;
use vlsir::circuit::{port, Concat, Connection, Instance, Module, Port, Signal, Slice};
use vlsir::reference::To;
use vlsir::Reference;

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
        println!("Splitting bits: {:?}", split);
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

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DecoderParams {
    pub tree: DecoderTree,
    pub lch: Int,
    pub name: String,
}

pub fn hierarchical_decoder(params: DecoderParams) -> Vec<Module> {
    let out = params.tree.root.num;
    let in_bits = log2(out) as i64;

    let ports = vec![
        Port {
            signal: Some(Signal {
                name: "vdd".to_string(),
                width: 1,
            }),
            direction: port::Direction::Inout as i32,
        },
        Port {
            signal: Some(Signal {
                name: "gnd".to_string(),
                width: 1,
            }),
            direction: port::Direction::Inout as i32,
        },
        Port {
            signal: Some(Signal {
                name: "addr".to_string(),
                width: in_bits,
            }),
            direction: port::Direction::Input as i32,
        },
        Port {
            signal: Some(Signal {
                name: "addr_b".to_string(),
                width: in_bits,
            }),
            direction: port::Direction::Input as i32,
        },
        Port {
            signal: Some(Signal {
                name: "decode".to_string(),
                width: out as i64,
            }),
            direction: port::Direction::Output as i32,
        },
        Port {
            signal: Some(Signal {
                name: "decode_b".to_string(),
                width: out as i64,
            }),
            direction: port::Direction::Output as i32,
        },
    ];

    let mut m = Module {
        name: params.name.clone(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    let vdd = signal("vdd");
    let gnd = signal("gnd");

    let mut gen = DecoderGen::new(&params, &vdd, &gnd, in_bits as usize);
    gen.helper(Some(&params.tree.root), 0);

    m.instances.append(&mut gen.instances);
    gen.modules.push(m);

    gen.modules
}

struct DecoderGen<'a> {
    ctr: usize,
    params: &'a DecoderParams,
    vdd: &'a Signal,
    gnd: &'a Signal,
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
        addr_bits: usize,
    ) -> Self {
        Self {
            ctr: 0,
            params,
            vdd,
            gnd,
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

    fn helper(&mut self, node: Option<&TreeNode>, depth: usize) -> BusConnection {
        if node.is_none() {
            assert!(self.addr_bits >= 1);
            self.addr_bits -= 1;
            let c = Connection {
                stype: Some(Stype::Concat(Concat {
                    parts: vec![
                        Connection {
                            stype: Some(Stype::Slice(Slice {
                                signal: "addr_b".to_string(),
                                top: self.addr_bits as i64,
                                bot: self.addr_bits as i64,
                            })),
                        },
                        Connection {
                            stype: Some(Stype::Slice(Slice {
                                signal: "addr".to_string(),
                                top: self.addr_bits as i64,
                                bot: self.addr_bits as i64,
                            })),
                        },
                    ],
                })),
            };
            return c.into();
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

        println!("sigs = {:?}, num = {}", sigs, node.num);
        assert_eq!(sigs.iter().map(|s| s.width()).product::<usize>(), node.num);

        let out_name = if depth == 0 {
            "decode".to_string()
        } else {
            format!("predecode_{}", self.get_id())
        };

        let out = Signal {
            name: out_name.clone(),
            width: node.num as i64,
        };

        let nand_name = if let Some(nand_name) = self.nands.get(&(gate_size, node.gate.size)) {
            nand_name.to_string()
        } else {
            let nand_name = format!("{}_nand_{}", &self.params.name, self.get_id());
            let nand = match gate_size {
                2 => nand2(GateParams {
                    name: nand_name.clone(),
                    size: node.gate.size,
                    length: self.params.lch,
                }),
                3 => nand3(GateParams {
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
            let inv = inv(GateParams {
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

            let mut conns: HashMap<_, _> = [
                ("vdd", sig_conn(self.vdd)),
                ("gnd", sig_conn(self.gnd)),
                ("y", sig_conn(&tmp)),
            ]
            .into();

            for (j, port) in ports.enumerate() {
                let conn = Connection {
                    stype: Some(Stype::Slice(sigs[j].get(idxs[j]).unwrap())),
                };
                conns.insert(port, conn);
            }

            let nand = Instance {
                name: format!("nand_{}", self.get_id()),
                module: Some(Reference {
                    to: Some(To::Local(nand_name.clone())),
                }),
                connections: conn_map(conns),
                ..Default::default()
            };
            self.instances.push(nand);

            let conns: HashMap<_, _> = [
                ("vdd", sig_conn(self.vdd)),
                ("gnd", sig_conn(self.gnd)),
                ("din", sig_conn(&tmp)),
                ("din_b", conn_slice(&out_name, i as i64, i as i64)),
            ]
            .into();

            let inv = Instance {
                name: format!("inv_{}", self.get_id()),
                module: Some(Reference {
                    to: Some(To::Local(inv_name.clone())),
                }),
                connections: conn_map(conns),
                ..Default::default()
            };
            self.instances.push(inv);
        }

        Connection {
            stype: Some(Stype::Sig(out)),
        }
        .into()
    }
}

pub struct Decoder24Params {
    pub gate_size: Size,
    pub inv_size: Size,
    pub lch: Int,
    pub name: String,
}

pub fn decoder_24(params: Decoder24Params) -> Vec<Module> {
    let nand_name = format!("{}_nand", &params.name);
    let nand = nand2(GateParams {
        name: nand_name.clone(),
        size: params.gate_size,
        length: params.lch,
    });

    let inv_name = format!("{}_inv", &params.name);
    let inv = inv(GateParams {
        name: inv_name.clone(),
        size: params.inv_size,
        length: params.lch,
    });

    let vdd = signal("vdd");
    let gnd = signal("gnd");
    let din = Signal {
        name: "din".to_string(),
        width: 2,
    };
    let din_b = Signal {
        name: "din_b".to_string(),
        width: 2,
    };
    let dout = Signal {
        name: "dout".to_string(),
        width: 4,
    };

    let ports = vec![
        Port {
            signal: Some(vdd.clone()),
            direction: port::Direction::Inout as i32,
        },
        Port {
            signal: Some(gnd.clone()),
            direction: port::Direction::Inout as i32,
        },
        Port {
            signal: Some(din),
            direction: port::Direction::Input as i32,
        },
        Port {
            signal: Some(din_b),
            direction: port::Direction::Input as i32,
        },
        Port {
            signal: Some(dout),
            direction: port::Direction::Output as i32,
        },
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..4 {
        let mut conns = HashMap::with_capacity(4);
        conns.insert(
            "vdd".to_string(),
            Connection {
                stype: Some(Stype::Sig(vdd.clone())),
            },
        );
        conns.insert(
            "gnd".to_string(),
            Connection {
                stype: Some(Stype::Sig(gnd.clone())),
            },
        );

        let mut inv_conns = conns.clone();

        let tmp = signal(format!("out_b_{}", i));
        conns.insert(
            "y".to_string(),
            Connection {
                stype: Some(Stype::Sig(tmp.clone())),
            },
        );

        let sig1 = match i & 0x1 {
            0 => "din",
            1 => "din_b",
            _ => unreachable!(),
        };
        let sig2 = match (i >> 1) & 0x1 {
            0 => "din",
            1 => "din_b",
            _ => unreachable!(),
        };

        let a = Connection {
            stype: Some(Stype::Slice(Slice {
                signal: sig1.to_string(),
                top: 0,
                bot: 0,
            })),
        };
        let b = Connection {
            stype: Some(Stype::Slice(Slice {
                signal: sig2.to_string(),
                top: 1,
                bot: 1,
            })),
        };
        conns.insert("a".to_string(), a);
        conns.insert("b".to_string(), b);
        let nand = Instance {
            name: format!("nand_{}", i),
            module: Some(Reference {
                to: Some(To::Local(nand_name.clone())),
            }),
            connections: conns,
            ..Default::default()
        };
        m.instances.push(nand);

        inv_conns.insert("din".to_string(), sig_conn(&tmp));
        inv_conns.insert(
            "din_b".to_string(),
            Connection {
                stype: Some(Stype::Slice(Slice {
                    signal: "dout".to_string(),
                    top: i,
                    bot: i,
                })),
            },
        );

        let inv = Instance {
            name: format!("inv_{}", i),
            module: Some(Reference {
                to: Some(To::Local(inv_name.clone())),
            }),
            connections: inv_conns,
            ..Default::default()
        };
        m.instances.push(inv);
    }

    vec![nand, inv, m]
}

/*
impl<'a> DecoderGen<'a> {
    fn generate(&mut self, node: &TreeNode) {
        let gate_name = format!("nand_dec_{}", self.depth);

        let x = match node.gate.gate_type {
            GateType::Nand2 => 1,
            GateType::Nand3 => 2,
            _ => panic!("unsupported gate type"),
        };
        let params = GateParams {
            name: format!("nand2_dec_{}", self.depth),
            length: self.lch,
            size: node.gate.size,
        };

        for i in 0..node.num {
            self.m.instances.push(Instance {
                name: format!("nand_{}_{}", self.depth, i),
                module: Some(Reference {
                    to: Some(To::Local(gate_name.clone())),
                }),

            });
        }
    }
}
*/
