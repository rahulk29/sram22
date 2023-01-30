use fanout::FanoutAnalyzer;
use serde::{Deserialize, Serialize};
use substrate::component::Component;

use super::gate::{GateParams, GateType, PrimitiveGateParams};

pub mod layout;
pub mod schematic;

pub struct Decoder {
    params: DecoderParams,
}

pub struct DecoderStage {
    params: DecoderStageParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecoderParams {
    pub tree: DecoderTree,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DecoderStageParams {
    pub gate: GateParams,
    pub buf: PrimitiveGateParams,
    pub num: usize,
    pub child_sizes: Vec<usize>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DecoderTree {
    pub root: TreeNode,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TreeNode {
    pub gate: GateParams,
    pub buf: PrimitiveGateParams,
    // Number of one-hot outputs.
    pub num: usize,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct PlanTreeNode {
    gate: GateType,
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
        f.add_gate(fanout::GateType::INV);
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
    let buf = PrimitiveGateParams {
        nwidth: 1_600,
        pwidth: 2_400,
        length: 150,
    };

    TreeNode {
        gate: GateParams::new_primitive(
            x.gate,
            PrimitiveGateParams {
                nwidth: 3_200,
                pwidth: 2_400,
                length: 150,
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
            num: 4,
            children: vec![],
        }
    } else if bits == 3 {
        PlanTreeNode {
            gate: GateType::Nand3,
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

pub(crate) fn get_idxs(mut num: usize, bases: &[usize]) -> Vec<usize> {
    let products = bases
        .iter()
        .rev()
        .scan(1, |state, &elem| {
            let val = *state;
            *state *= elem;
            Some(val)
        })
        .collect::<Vec<_>>();
    let mut idxs = Vec::with_capacity(bases.len());

    for i in 0..bases.len() {
        let j = products.len() - i - 1;
        idxs.push(num / products[j]);
        num %= products[j];
    }
    idxs
}

impl Component for Decoder {
    type Params = DecoderParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("decoder")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for DecoderStage {
    type Params = DecoderStageParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("decoder_stage")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::out_spice;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    fn test_decode_4bit() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_decoder_4bit");

        let tree = DecoderTree::new(4);
        let params = DecoderParams { tree };

        ctx.write_schematic_to_file::<Decoder>(&params, out_spice(work_dir, "netlist"))
            .expect("failed to write schematic");
    }
}
