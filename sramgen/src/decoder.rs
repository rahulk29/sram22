use fanout::FanoutAnalyzer;

use crate::gate::{Gate, GateType, Size};

pub struct DecoderTree {
    pub root: TreeNode,
}

pub struct TreeNode {
    pub gate: Gate,
    pub buf: Option<Gate>,
    pub num: usize,
    pub children: Vec<TreeNode>,
}

struct PlanTreeNode {
    gate: GateType,
    buf: Option<GateType>,
    num: usize,
    children: Vec<PlanTreeNode>,
}

impl DecoderTree {
    pub fn new(bits: usize) -> Self {
        let plan = plan_decoder(bits);
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
            f.add_branch(next.num as f64);
        }
    }
    // TODO use fanout results

    size_helper_tmp(tree)
}

fn size_helper_tmp(x: &PlanTreeNode) -> TreeNode {
    let buf = x.buf.map(|b| {
        Gate::new(
            b,
            Size {
                nmos_width: 1_000,
                pmos_width: 1_000,
            },
        )
    });
    TreeNode {
        gate: Gate::new(
            x.gate,
            Size {
                nmos_width: 1_000,
                pmos_width: 1_000,
            },
        ),
        buf,
        num: x.num,
        children: x.children.iter().map(size_helper_tmp).collect::<Vec<_>>(),
    }
}

fn plan_decoder(bits: usize) -> PlanTreeNode {
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
        let split = partition_bits(bits);
        let gate = match split.len() {
            2 => GateType::Nand2,
            3 => GateType::Nand3,
            _ => panic!("unexpected bit split"),
        };

        let children = split
            .into_iter()
            .map(|b| plan_decoder(b))
            .collect::<Vec<_>>();
        PlanTreeNode {
            gate,
            buf: Some(GateType::Inv),
            num: 2usize.pow(bits as u32),
            children,
        }
    }
}

fn partition_bits(bits: usize) -> Vec<usize> {
    assert!(bits > 3);

    if bits % 2 == 0 {
        vec![bits / 2, bits / 2]
    } else {
        match bits % 3 {
            0 => vec![bits / 3, bits / 3, bits / 3],
            1 => vec![bits / 3 + 1, bits / 3, bits / 3],
            2 => vec![bits / 3 + 1, bits / 3 + 1, bits / 3],
            _ => panic!("unexpected remainder of `bits` divided by 3"),
        }
    }
}
