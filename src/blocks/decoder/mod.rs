use self::layout::{
    decoder_stage_layout, LastBitDecoderPhysicalDesignScript, PredecoderPhysicalDesignScript,
    RoutingStyle,
};
use crate::blocks::decoder::sizing::{path_map_tree, Tree, ValueTree};
use serde::{Deserialize, Serialize};
use subgeom::snap_to_grid;
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::logic::delay::{GateModel, LogicPath};
use substrate::schematic::circuit::Direction;

use super::gate::{AndParams, Gate, GateParams, GateType, PrimitiveGateParams};

pub mod layout;
pub mod schematic;
pub mod sim;

pub mod sizing;

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
    pub child_nums: Vec<usize>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct PlanTreeNode {
    gate: GateType,
    num: usize,
    children: Vec<PlanTreeNode>,
    skew_rising: bool,
    cols: bool,
}

pub struct WlDriver {
    params: DecoderStageParams,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AddrGateParams {
    pub gate: GateParams,
    pub num: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AddrGate {
    params: AddrGateParams,
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

impl Component for AddrGate {
    type Params = AddrGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        let gate = match params.gate {
            params @ GateParams::And2(_) => params,
            GateParams::And3(params) => GateParams::And2(params),
            _ => panic!("Unsupported wmux driver gate"),
        };
        Ok(Self {
            params: AddrGateParams {
                gate,
                num: params.num,
            },
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("addr_gate")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let n = self.params.num;
        let vdd = ctx.port("vdd", Direction::InOut);
        let addr = ctx.bus_port("addr", n, Direction::Input);
        let addr_b = ctx.bus_port("addr_b", n, Direction::Input);
        let en = ctx.port("en", Direction::Input);
        let y = ctx.bus_port("addr_gated", n, Direction::Output);
        let yb = ctx.bus_port("addr_b_gated", n, Direction::Output);
        let vss = ctx.port("vss", Direction::InOut);

        let [int1, int2] = ctx.buses(["int1", "int2"], n);

        for i in 0..n {
            ctx.instantiate::<Gate>(&self.params.gate)?
                .with_connections([
                    ("vdd", vdd),
                    ("a", addr.index(i)),
                    ("b", en),
                    ("y", y.index(i)),
                    ("yb", int1.index(i)),
                    ("vss", vss),
                ])
                .named(format!("addr_gate_{i}"))
                .add_to(ctx);
            ctx.instantiate::<Gate>(&self.params.gate)?
                .with_connections([
                    ("vdd", vdd),
                    ("a", addr_b.index(i)),
                    ("b", en),
                    ("y", yb.index(i)),
                    ("yb", int2.index(i)),
                    ("vss", vss),
                ])
                .named(format!("addr_b_gate_{i}"))
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
        let params = DecoderStageParams {
            gate: self.params.gate,
            num: self.params.num,
            child_sizes: vec![],
        };
        decoder_stage_layout(ctx, &params, &dsn, RoutingStyle::Driver)
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
        let gate = match params.gate {
            params @ GateParams::And2(_) => params,
            GateParams::And3(params) => GateParams::And2(params),
            _ => panic!("Unsupported wmux driver gate"),
        };
        Ok(Self {
            params: DecoderStageParams {
                gate,
                num: params.num,
                child_sizes: params.child_sizes.clone(),
            },
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
    pub fn for_columns(bits: usize, top_scale: i64) -> Self {
        let plan = plan_decoder(bits, true, false, true);
        let mut root = size_decoder(&plan);
        root.gate = root.gate.scale(top_scale);
        DecoderTree { root }
    }

    pub fn with_scale_and_skew(bits: usize, top_scale: i64, skew_rising: bool) -> Self {
        let plan = plan_decoder(bits, true, skew_rising, false);
        let mut root = size_decoder(&plan);
        root.gate = root.gate.scale(top_scale);
        DecoderTree { root }
    }

    #[inline]
    pub fn with_scale(bits: usize, top_scale: i64) -> Self {
        Self::with_scale_and_skew(bits, top_scale, false)
    }

    #[inline]
    pub fn with_skew(bits: usize, skew_rising: bool) -> Self {
        Self::with_scale_and_skew(bits, 1, skew_rising)
    }

    #[inline]
    pub fn new(bits: usize) -> Self {
        Self::with_scale_and_skew(bits, 1, false)
    }
}

fn size_decoder(tree: &PlanTreeNode) -> TreeNode {
    // TODO improve decoder sizing
    size_helper_tmp(tree, tree.skew_rising, tree.cols)
}

const GATE_MODEL: GateModel = GateModel {
    res: 1.0,
    cin: 1.0,
    cout: 1.0,
};

const NAND_PARAMS: PrimitiveGateParams = PrimitiveGateParams {
    nwidth: 2_000,
    pwidth: 1_600,
    length: 150,
};

fn gate_model(gate: GateType) -> GateModel {
    GATE_MODEL
}

fn scale(gate: PrimitiveGateParams, scale: f64) -> PrimitiveGateParams {
    let nwidth = snap_to_grid((gate.nwidth as f64 * scale).round() as i64, 10);
    let pwidth = snap_to_grid((gate.pwidth as f64 * scale).round() as i64, 10);
    PrimitiveGateParams {
        nwidth,
        pwidth,
        length: gate.length,
    }
}

fn size_path(path: &[&PlanTreeNode], end: &f64) -> TreeNode {
    let mut lp = LogicPath::new();
    let mut vars = Vec::new();
    for node in path.iter().copied().rev() {
        for gate in node.gate.primitive_gates() {
            let var = lp.create_variable_with_initial(1.);
            lp.append_unsized_gate(GATE_MODEL, var);
            vars.push(var);
        }
    }

    lp.size();

    let mut cnode: Option<&mut TreeNode> = None;
    let mut tree = None;
    let mut vars = vars.iter().copied();

    for &node in path {
        let gate = match node.gate {
            GateType::And2 => GateParams::And2(AndParams {
                nand: scale(NAND_PARAMS, lp.value(vars.next().unwrap())),
                inv: scale(NAND_PARAMS, lp.value(vars.next().unwrap())),
            }),
            _ => todo!(),
        };

        let n = TreeNode {
            gate,
            num: node.num,
            children: vec![],
            child_nums: node.children.iter().map(|n| n.num).collect(),
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

impl Tree for PlanTreeNode {
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

impl Tree for TreeNode {
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

impl ValueTree<f64> for TreeNode {
    fn value_for_child(&self, idx: usize) -> f64 {
        (self.num / self.child_nums[idx]) as f64 * gate_model(self.gate.gate_type()).cin
    }
}

fn size_helper_tmp(x: &PlanTreeNode, skew_rising: bool, cols: bool) -> TreeNode {
    path_map_tree(x, &size_path, &1.)
}

fn plan_decoder(bits: usize, top: bool, skew_rising: bool, cols: bool) -> PlanTreeNode {
    assert!(bits > 1);
    if bits == 2 {
        PlanTreeNode {
            gate: GateType::And2,
            num: 4,
            children: vec![],
            skew_rising,
            cols,
        }
    } else if bits == 3 {
        PlanTreeNode {
            gate: GateType::And3,
            num: 8,
            children: vec![],
            skew_rising,
            cols,
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
            .map(|x| plan_decoder(x, false, skew_rising, cols))
            .collect::<Vec<_>>();
        PlanTreeNode {
            gate,
            num: 2usize.pow(bits as u32),
            children,
            skew_rising,
            cols,
        }
    }
}

fn partition_bits(bits: usize, top: bool) -> Vec<usize> {
    assert!(bits > 3);

    if top {
        let right = bits / 2;
        return vec![bits - right, right];
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
        let right = bits / 2;
        vec![bits - right, right]
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

    use crate::blocks::gate::AndParams;
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

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
                    nwidth: 2_000,
                    pwidth: 2_000,
                    length: 150,
                },
                inv: PrimitiveGateParams {
                    nwidth: 1_000,
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
        #[cfg(feature = "commercial")]
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
        #[cfg(feature = "commercial")]
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
