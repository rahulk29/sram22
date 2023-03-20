use fanout::FanoutAnalyzer;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;

use self::layout::{
    decoder_stage_layout, LastBitDecoderPhysicalDesignScript, PredecoderPhysicalDesignScript,
    RoutingStyle,
};

use super::gate::{AndParams, Gate, GateParams, GateType, PrimitiveGateParams};

pub mod layout;
pub mod schematic;

pub struct Decoder {
    params: DecoderParams,
}

pub struct Predecoder {
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

pub struct WlDriver {
    params: DecoderStageParams,
}

impl Component for WlDriver {
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
        arcstr::literal!("wordline_driver")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let n = self.params.num;
        let vdd = ctx.port("vdd", Direction::InOut);
        let input = ctx.bus_port("in", n, Direction::Input);
        let en = ctx.port("wl_en", Direction::Input);
        let y = ctx.bus_port("decode", n, Direction::Output);
        let yb = ctx.bus_port("decode_b", n, Direction::Output);
        let vss = ctx.port("vss", Direction::InOut);
        for i in 0..n {
            ctx.instantiate::<Gate>(&self.params.gate)?
                .with_connections([
                    ("vdd", vdd),
                    ("a", input.index(i)),
                    ("b", en),
                    ("y", y.index(i)),
                    ("yb", yb.index(i)),
                    ("vss", vss),
                ])
                .named(format!("gate_{i}"))
                .add_to(ctx);
        }
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<LastBitDecoderPhysicalDesignScript>(&NoParams)?;
        decoder_stage_layout(ctx, &self.params, &dsn, RoutingStyle::Driver)
    }
}

pub struct WmuxDriver {
    params: DecoderStageParams,
}

impl Component for WmuxDriver {
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
        arcstr::literal!("wmux_driver")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let n = self.params.num;
        let vdd = ctx.port("vdd", Direction::InOut);
        let input = ctx.bus_port("in", n, Direction::Input);
        let en = ctx.port("en", Direction::Input);
        let y = ctx.bus_port("decode", n, Direction::Output);
        let yb = ctx.bus_port("decode_b", n, Direction::Output);
        let vss = ctx.port("vss", Direction::InOut);
        for i in 0..n {
            ctx.instantiate::<Gate>(&self.params.gate)?
                .with_connections([
                    ("vdd", vdd),
                    ("a", input.index(i)),
                    ("b", en),
                    ("y", y.index(i)),
                    ("yb", yb.index(i)),
                    ("vss", vss),
                ])
                .named(format!("gate_{i}"))
                .add_to(ctx);
        }
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx
            .inner()
            .run_script::<PredecoderPhysicalDesignScript>(&NoParams)?;
        decoder_stage_layout(ctx, &self.params, &dsn, RoutingStyle::Driver)
    }
}

impl DecoderTree {
    pub fn with_scale(bits: usize, top_scale: i64) -> Self {
        let mut tree = Self::new(bits);
        tree.root.gate = tree.root.gate.scale(top_scale);
        tree
    }

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

    size_helper_tmp(tree, &sizes)
}

fn size_helper_tmp(x: &PlanTreeNode, _sizes: &[f64]) -> TreeNode {
    // TODO size decoder
    TreeNode {
        gate: GateParams::new_and(
            x.gate,
            AndParams {
                nand: PrimitiveGateParams {
                    nwidth: 3_200,
                    pwidth: 2_400,
                    length: 150,
                },
                inv: PrimitiveGateParams {
                    nwidth: 2_000,
                    pwidth: 2_000,
                    length: 150,
                },
            },
        ),
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
            gate: GateType::And2,
            num: 4,
            children: vec![],
        }
    } else if bits == 3 {
        PlanTreeNode {
            gate: GateType::And3,
            num: 8,
            children: vec![],
        }
    } else {
        let split = partition_bits(bits, top);
        let gate = match split.len() {
            2 => GateType::And2,
            3 => GateType::And3,
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
}

impl Component for Predecoder {
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
        arcstr::literal!("predecoder")
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

    use substrate::component::NoParams;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use crate::v2::gate::AndParams;

    use super::layout::{
        DecoderGate, DecoderGateParams, LastBitDecoderPhysicalDesignScript, LastBitDecoderStage,
    };
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

    #[test]
    fn test_decoder_stage() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_decoder_stage_4");

        let params = DecoderStageParams {
            gate: GateParams::And2(AndParams {
                nand: PrimitiveGateParams {
                    nwidth: 3_000,
                    pwidth: 1_200,
                    length: 150,
                },
                inv: PrimitiveGateParams {
                    nwidth: 2_000,
                    pwidth: 2_000,
                    length: 150,
                },
            }),
            num: 4,
            child_sizes: vec![2, 2],
        };

        ctx.write_layout::<DecoderStage>(&params, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_decoder_gate() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_decoder_gate");

        let dsn = ctx
            .run_script::<LastBitDecoderPhysicalDesignScript>(&NoParams)
            .expect("failed to run design script");

        let params = DecoderGateParams {
            gate: GateParams::And2(AndParams {
                nand: PrimitiveGateParams {
                    nwidth: 3_000,
                    pwidth: 1_200,
                    length: 150,
                },
                inv: PrimitiveGateParams {
                    nwidth: 2_000,
                    pwidth: 2_000,
                    length: 150,
                },
            }),
            dsn: (*dsn).clone(),
        };

        ctx.write_layout::<DecoderGate>(&params, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_predecoder_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_predecoder_4");

        let tree = DecoderTree::new(4);
        let params = DecoderParams { tree };

        ctx.write_layout::<Predecoder>(&params, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_predecoder_6() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_predecoder_6");

        let tree = DecoderTree::new(6);
        let params = DecoderParams { tree };

        ctx.write_layout::<Predecoder>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        #[cfg(feature = "calibre")]
        {
            let output = ctx
                .write_drc::<Predecoder>(&params, work_dir.join("drc"))
                .expect("failed to run drc");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
        }
    }

    #[test]
    fn test_last_bit_decoder_stage() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_last_bit_decoder_4");

        let params = DecoderStageParams {
            gate: GateParams::And2(AndParams {
                nand: PrimitiveGateParams {
                    nwidth: 3_000,
                    pwidth: 1_200,
                    length: 150,
                },
                inv: PrimitiveGateParams {
                    nwidth: 2_000,
                    pwidth: 2_000,
                    length: 150,
                },
            }),
            num: 16,
            child_sizes: vec![4, 4],
        };

        ctx.write_layout::<LastBitDecoderStage>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        #[cfg(feature = "calibre")]
        {
            let output = ctx
                .write_drc::<LastBitDecoderStage>(&params, work_dir.join("drc"))
                .expect("failed to run drc");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
        }
    }
}
