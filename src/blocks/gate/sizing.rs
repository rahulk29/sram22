use crate::blocks::gate::{Gate, PrimitiveGateType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct InverterGateTreeNode {
    gate: PrimitiveGateType,
    id: u64,
    /// The number of inverters placed after `gate`.
    n_invs: usize,
    /// The number of gates in the next stage
    /// that the final gate associated to this node drives.
    n_branching: usize,
    children: Vec<InverterGateTreeNode>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateTree {
    root: GateTreeNode,
    load_cap: f64,
}

impl InverterGateTreeNode {
    pub fn elaborate(&self) -> GateTreeNode {
        let leaf = GateTreeNode {
            gate: self.gate,
            id: self.id,
            n_branching: 1,
            children: vec![],
        };

        for child in self.children.iter() {
            let child = child.elaborate();
        }

        leaf
    }
}
