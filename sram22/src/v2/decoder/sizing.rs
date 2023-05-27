use super::*;

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
        for gate in node.gate.as_fanout_gates() {
            f.add_gate(gate);
        }
        if let Some(next) = nodes.get(i + 1) {
            f.add_branch((next.num / node.num) as f64);
        }
    }
    // TODO use fanout results
    let res = f.size(32f64);
    let mut sizes = res.sizes().collect::<Vec<_>>();

    sizes.reverse();

    size_helper_tmp(tree, &sizes, tree.skew_rising)
}

struct SizingParams {
    lch: i64,
    /// Logical effort of a NAND2 gate.
    g_nand2: f64,
    /// Logical effort of a NAND3 gate.
    g_nand3: f64,
    /// Input capacitance of a unit inverter.
    c1: f64,
}

fn size_path(path: &[&PlanTreeNode]) -> Vec<f64> {

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan() {
        for bits in [5, 8, 10] {
            let tree = plan_decoder(bits, true, true);
            println!("{:?}", tree);
        }
    }
}
