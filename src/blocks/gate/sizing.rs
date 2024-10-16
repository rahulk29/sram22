use crate::blocks::decoder::sizing::{path_map_tree, Tree, ValueTree};
use crate::blocks::decoder::{primitive_gate_model, primitive_gate_params};
use crate::blocks::gate::{PrimitiveGateParams, PrimitiveGateType};
use serde::{Deserialize, Serialize};
use substrate::logic::delay::{LogicPath, OptimizerOpts};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct InverterGateTreeNode {
    pub(crate) gate: PrimitiveGateType,
    pub(crate) id: u64,
    /// The number of inverters placed after `gate`.
    pub(crate) n_invs: usize,
    /// The number of gates in the next stage
    /// that the final gate associated to this node drives.
    pub(crate) n_branching: usize,
    pub(crate) children: Vec<InverterGateTreeNode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GateTreeNode {
    gate: PrimitiveGateType,
    id: u64,
    /// The number of gates in the next stage
    /// that the final gate associated to this node drives.
    n_branching: usize,
    children: Vec<GateTreeNode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SizedGateTreeNode {
    gate: PrimitiveGateParams,
    gate_type: PrimitiveGateType,
    id: u64,
    /// The number of gates in the next stage
    /// that the final gate associated to this node drives.
    n_branching: usize,
    children: Vec<SizedGateTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateTree {
    root: GateTreeNode,
    load_cap: f64,
}

impl InverterGateTreeNode {
    pub fn elaborate(&self) -> GateTreeNode {
        elaborate_inner(self, self.n_invs, self.n_branching)
    }

    pub fn buffer(stages: usize) -> Self {
        assert!(stages >= 2);
        assert_eq!(stages % 2, 0);
        Self {
            gate: PrimitiveGateType::Inv,
            id: 1,
            n_invs: stages - 1,
            n_branching: 1,
            children: vec![],
        }
    }

    pub fn inverter(stages: usize) -> Self {
        assert!(stages >= 1);
        assert_eq!(stages % 2, 1);
        Self {
            gate: PrimitiveGateType::Inv,
            id: 1,
            n_invs: stages - 1,
            n_branching: 1,
            children: vec![],
        }
    }
}

fn elaborate_inner(node: &InverterGateTreeNode, n_invs: usize, n_branching: usize) -> GateTreeNode {
    if n_invs == 0 {
        GateTreeNode {
            gate: node.gate,
            id: node.id,
            n_branching,
            children: node.children.iter().map(|n| n.elaborate()).collect(),
        }
    } else {
        GateTreeNode {
            gate: PrimitiveGateType::Inv,
            id: node.id,
            n_branching,
            children: vec![elaborate_inner(node, n_invs - 1, 1)],
        }
    }
}

impl GateTreeNode {
    pub fn size(&self, cl: f64) -> SizedGateTreeNode {
        path_map_tree(self, &size_path, &cl)
    }
}

impl SizedGateTreeNode {
    pub fn as_inv_chain(&self) -> Vec<PrimitiveGateParams> {
        let mut invs = Vec::new();
        let mut node = self;

        loop {
            assert_eq!(node.gate_type, PrimitiveGateType::Inv);
            invs.push(node.gate);
            if node.children.is_empty() {
                break;
            }
            assert_eq!(node.children.len(), 1);
            node = &node.children[0];
        }

        invs.iter().copied().rev().collect()
    }

    pub fn as_chain(&self) -> Vec<PrimitiveGateParams> {
        let mut sizes = Vec::new();
        let mut node = self;

        loop {
            sizes.push(node.gate);
            if node.children.is_empty() {
                break;
            }
            assert_eq!(node.children.len(), 1);
            node = &node.children[0];
        }

        sizes.iter().copied().rev().collect()
    }
}

fn size_path(path: &[&GateTreeNode], end: &f64) -> SizedGateTreeNode {
    let mut lp = LogicPath::new();
    let mut vars = Vec::new();
    for (i, node) in path.iter().copied().rev().enumerate() {
        let gate = node.gate;
        if i == 0 {
            lp.append_sized_gate(primitive_gate_model(gate));
        } else {
            let var = lp.create_variable_with_initial(2.);
            let model = primitive_gate_model(gate);
            let branching = path[path.len() - 1 - i].n_branching - 1;
            if branching > 0 {
                let mult = branching as f64 * model.cin;
                assert!(mult >= 0.0, "mult must be larger than zero, got {mult}");
                lp.append_variable_capacitor(mult, var);
            }

            lp.append_unsized_gate(model, var);
            vars.push(var);
        }
    }

    lp.append_capacitor(path[0].n_branching as f64 * *end);

    lp.size_with_opts(OptimizerOpts {
        lr: 1e11,
        lr_decay: 0.999999,
        max_iter: 10_000_000,
    });

    let mut cnode: Option<&mut SizedGateTreeNode> = None;
    let mut tree = None;

    let mut values = vars
        .iter()
        .rev()
        .map(|v| {
            let v = lp.value(*v);
            if v < 0.5 {
                0.5
            } else {
                v
            }
        })
        .collect::<Vec<_>>();
    values.push(1.);
    let mut values = values.into_iter();

    for &node in path {
        let gate = match node.gate {
            PrimitiveGateType::Inv
            | PrimitiveGateType::FoldedInv
            | PrimitiveGateType::MultiFingerInv => crate::blocks::decoder::scale(
                crate::blocks::decoder::INV_PARAMS,
                values.next().unwrap(),
            ),
            PrimitiveGateType::Nand2 => crate::blocks::decoder::scale(
                crate::blocks::decoder::NAND2_PARAMS,
                values.next().unwrap(),
            ),
            PrimitiveGateType::Nand3 => crate::blocks::decoder::scale(
                crate::blocks::decoder::NAND3_PARAMS,
                values.next().unwrap(),
            ),
            PrimitiveGateType::Nor2 => crate::blocks::decoder::scale(
                crate::blocks::decoder::NOR2_PARAMS,
                values.next().unwrap(),
            ),
        };

        let n = SizedGateTreeNode {
            gate,
            gate_type: node.gate,
            id: node.id,
            n_branching: node.n_branching,
            children: vec![],
        };

        if let Some(parent) = cnode {
            parent.children.push(n);
            cnode = Some(&mut parent.children[0])
        } else {
            tree = Some(n);
            cnode = Some(tree.as_mut().unwrap());
        }
    }

    tree.unwrap()
}

impl Tree for GateTreeNode {
    fn children(&self) -> &[Self] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Self] {
        &mut self.children
    }

    fn add_right_child(&mut self, child: Self) {
        self.children.push(child);
    }
}

impl Tree for SizedGateTreeNode {
    fn children(&self) -> &[Self] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Self] {
        &mut self.children
    }

    fn add_right_child(&mut self, child: Self) {
        self.children.push(child);
    }
}

impl ValueTree<f64> for SizedGateTreeNode {
    fn value_for_child(&self, _idx: usize) -> f64 {
        let model = primitive_gate_model(self.gate_type);
        model.cin * self.gate.nwidth as f64 / (primitive_gate_params(self.gate_type).nwidth as f64)
    }
}
