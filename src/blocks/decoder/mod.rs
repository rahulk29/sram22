use self::layout::{
    decoder_stage_layout, LastBitDecoderPhysicalDesignScript, PredecoderPhysicalDesignScript,
    RoutingStyle,
};
use crate::blocks::decoder::sizing::{path_map_tree, Tree, ValueTree};
use serde::{Deserialize, Serialize};
use subgeom::snap_to_grid;
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::logic::delay::{GateModel, LogicPath, OptimizerOpts};
use substrate::schematic::circuit::Direction;

use super::gate::{AndParams, Gate, GateParams, GateType, PrimitiveGateParams, PrimitiveGateType};

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
    pub max_width: Option<i64>,
    pub gate: GateParams,
    pub invs: Vec<PrimitiveGateParams>,
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

impl Component for AddrGate {
    type Params = AddrGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        let gate = match params.gate {
            params @ GateParams::And2(_) => params,
            GateParams::And3(params) => GateParams::And2(params),
            x => panic!(
                "address gating must be performed by AND gates, got {:?}",
                x.gate_type()
            ),
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
            max_width: None,
            gate: self.params.gate,
            invs: vec![],
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
                max_width: None,
                gate,
                invs: vec![],
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
    pub fn new(bits: usize, cload: f64) -> Self {
        let plan = plan_decoder(bits, true, false);
        let mut root = size_decoder(&plan, cload);
        DecoderTree { root }
    }
}

fn size_decoder(tree: &PlanTreeNode, cwl: f64) -> TreeNode {
    path_map_tree(tree, &size_path, &cwl)
}

/// The on-resistance and capacitances of a 1x inverter ([`INV_PARAMS`]).
pub(crate) const INV_MODEL: GateModel = GateModel {
    res: 1422.118502462849,
    cin: 0.000000000000004482092764998187,
    cout: 0.0000000004387405174617657,
};

/// The on-resistance and capacitances of a 1x NAND2 gate ([`NAND2_PARAMS`]).
pub(crate) const NAND2_MODEL: GateModel = GateModel {
    res: 1478.364147093855,
    cin: 0.000000000000005389581112035269,
    cout: 0.0000000002743620195248461,
};

/// The on-resistance and capacitances of a 1x NAND3 gate ([`NAND3_PARAMS`]).
pub(crate) const NAND3_MODEL: GateModel = GateModel {
    res: 1478.037783669641,
    cin: 0.000000000000006217130454627972,
    cout: 0.000000000216366152882086,
};

/// The on-resistance and capacitances of a 1x NOR2 gate ([`NOR2_PARAMS`]).
pub(crate) const NOR2_MODEL: GateModel = GateModel {
    res: 1.0,
    cin: 1.0,
    cout: 1.0,
};

/// The sizing of a 1x inverter.
pub(crate) const INV_PARAMS: PrimitiveGateParams = PrimitiveGateParams {
    nwidth: 1_000,
    pwidth: 2_500,
    length: 150,
};

/// The sizing of a 1x NAND2 gate.
pub(crate) const NAND2_PARAMS: PrimitiveGateParams = PrimitiveGateParams {
    nwidth: 2_000,
    pwidth: 2_500,
    length: 150,
};

/// The sizing of a 1x NAND3 gate.
pub(crate) const NAND3_PARAMS: PrimitiveGateParams = PrimitiveGateParams {
    nwidth: 3_000,
    pwidth: 2_500,
    length: 150,
};

/// The sizing of a 1x NOR2 gate.
pub(crate) const NOR2_PARAMS: PrimitiveGateParams = PrimitiveGateParams {
    nwidth: 1_000,
    pwidth: 3_200,
    length: 150,
};

pub(crate) fn gate_params(gate: GateType) -> PrimitiveGateParams {
    match gate {
        GateType::Inv => INV_PARAMS,
        GateType::Nand2 => NAND2_PARAMS,
        GateType::Nand3 => NAND3_PARAMS,
        GateType::Nor2 => NOR2_PARAMS,
        gate => panic!("unsupported gate type: {gate:?}"),
    }
}

pub(crate) fn primitive_gate_params(gate: PrimitiveGateType) -> PrimitiveGateParams {
    match gate {
        PrimitiveGateType::Inv => INV_PARAMS,
        PrimitiveGateType::Nand2 => NAND2_PARAMS,
        PrimitiveGateType::Nand3 => NAND3_PARAMS,
        PrimitiveGateType::Nor2 => NOR2_PARAMS,
    }
}

pub(crate) fn gate_model(gate: GateType) -> GateModel {
    match gate {
        GateType::Inv => INV_MODEL,
        GateType::Nand2 => NAND2_MODEL,
        GateType::Nand3 => NAND3_MODEL,
        GateType::Nor2 => NOR2_MODEL,
        gate => panic!("unsupported gate type: {gate:?}"),
    }
}

pub(crate) fn primitive_gate_model(gate: PrimitiveGateType) -> GateModel {
    match gate {
        PrimitiveGateType::Inv => INV_MODEL,
        PrimitiveGateType::Nand2 => NAND2_MODEL,
        PrimitiveGateType::Nand3 => NAND3_MODEL,
        PrimitiveGateType::Nor2 => NOR2_MODEL,
    }
}

pub(crate) fn scale(gate: PrimitiveGateParams, scale: f64) -> PrimitiveGateParams {
    let nwidth = snap_to_grid((gate.nwidth as f64 * scale).round() as i64, 50);
    let pwidth = snap_to_grid((gate.pwidth as f64 * scale).round() as i64, 50);
    PrimitiveGateParams {
        nwidth,
        pwidth,
        length: gate.length,
    }
}

fn size_path(path: &[&PlanTreeNode], end: &f64) -> TreeNode {
    let mut lp = LogicPath::new();
    let mut vars = Vec::new();
    for (i, node) in path.iter().copied().rev().enumerate() {
        for (j, gate) in node.gate.primitive_gates().iter().copied().enumerate() {
            if i == 0 && j == 0 {
                lp.append_sized_gate(gate_model(gate));
            } else {
                let var = lp.create_variable_with_initial(2.);
                let model = gate_model(gate);
                if i != 0 && j == 0 {
                    let branching = node.num / node.children[0].num - 1;
                    if branching > 0 {
                        let mult = branching as f64 * model.cin;
                        assert!(mult >= 0.0, "mult must be larger than zero, got {mult}");
                        lp.append_variable_capacitor(mult, var);
                    }
                }
                lp.append_unsized_gate(model, var);
                vars.push(var);
            }
        }
    }
    lp.append_capacitor(*end);

    lp.size_with_opts(OptimizerOpts {
        lr: 1e11,
        lr_decay: 0.999995,
        max_iter: 10_000_000,
    });

    let mut cnode: Option<&mut TreeNode> = None;
    let mut tree = None;

    let mut values = vars
        .iter()
        .rev()
        .map(|v| {
            let v = lp.value(*v);
            assert!(v >= 0.5, "gate scale must be at least 0.5, got {v:.3}");
            v
        })
        .collect::<Vec<_>>();
    values.push(1.);
    let mut values = values.into_iter();

    for &node in path {
        let gate = match node.gate {
            GateType::And2 => GateParams::And2(AndParams {
                inv: scale(INV_PARAMS, values.next().unwrap()),
                nand: scale(NAND2_PARAMS, values.next().unwrap()),
            }),
            GateType::And3 => GateParams::And3(AndParams {
                inv: scale(INV_PARAMS, values.next().unwrap()),
                nand: scale(NAND3_PARAMS, values.next().unwrap()),
            }),
            GateType::Inv => GateParams::Inv(scale(INV_PARAMS, values.next().unwrap())),
            GateType::Nand2 => GateParams::Nand2(scale(NAND2_PARAMS, values.next().unwrap())),
            GateType::Nand3 => GateParams::Nand3(scale(NAND3_PARAMS, values.next().unwrap())),
            GateType::Nor2 => GateParams::Nor2(scale(NOR2_PARAMS, values.next().unwrap())),
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
        let first_gate_type = self.gate.gate_type().primitive_gates()[0];
        let first_gate = self.gate.first_gate_sizing();
        let model = gate_model(first_gate_type);
        (self.num / self.child_nums[idx]) as f64 * model.cin * first_gate.nwidth as f64
            / (gate_params(first_gate_type).nwidth as f64)
    }
}

fn plan_decoder(bits: usize, top: bool, skew_rising: bool) -> PlanTreeNode {
    assert!(bits > 1);
    if bits == 2 {
        PlanTreeNode {
            gate: GateType::And2,
            num: 4,
            children: vec![],
            skew_rising,
        }
    } else if bits == 3 {
        PlanTreeNode {
            gate: GateType::And3,
            num: 8,
            children: vec![],
            skew_rising,
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
            .map(|x| plan_decoder(x, false, skew_rising))
            .collect::<Vec<_>>();
        let node = PlanTreeNode {
            gate,
            num: 2usize.pow(bits as u32),
            children,
            skew_rising,
        };

        if top {
            PlanTreeNode {
                gate: GateType::Inv,
                num: node.num,
                children: vec![PlanTreeNode {
                    gate: GateType::Inv,
                    num: node.num,
                    children: vec![node],
                    skew_rising,
                }],
                skew_rising,
            }
        } else {
            node
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

        let tree = DecoderTree::new(4, 150e-15);
        let params = DecoderParams { tree };

        ctx.write_schematic_to_file::<Decoder>(&params, out_spice(work_dir, "netlist"))
            .expect("failed to write schematic");
    }

    #[test]
    fn test_decoder_stage() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_decoder_stage_4");

        let params = DecoderStageParams {
            max_width: None,
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
            invs: vec![],
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
            gate: Some(GateParams::And2(AndParams {
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
            })),
            dsn: (*dsn).clone(),
        };

        ctx.write_layout::<DecoderGate>(&params, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_predecoder_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_predecoder_4");

        let tree = DecoderTree::new(4, 150e-15);
        let params = DecoderParams { tree };

        ctx.write_layout::<Predecoder>(&params, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_predecoder_6() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_predecoder_6");

        let tree = DecoderTree::new(6, 150e-15);
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
            max_width: Some(70_000),
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
            invs: vec![PrimitiveGateParams {
                nwidth: 2_000,
                pwidth: 2_000,
                length: 150,
            }],
            num: 16,
            child_sizes: vec![4, 4],
        };

        let spice_path = out_spice(&work_dir, "schematic");
        ctx.write_schematic_to_file::<LastBitDecoderStage>(&params, &spice_path)
            .expect("failed to write schematic");

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

            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<LastBitDecoderStage>(&params, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }
}
