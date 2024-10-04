use std::collections::{HashMap, HashSet};

use arcstr::ArcStr;
use serde::{Deserialize, Serialize};
use substrate::component::Component;
use substrate::layout::cell::{CellPort, PortConflictStrategy};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignMode;
use substrate::layout::placement::array::ArrayTiler;

use super::decoder::layout::{DecoderGate, DecoderGateParams, DecoderTap};
use super::decoder::{self};

pub mod layout;
pub mod schematic;
pub mod sizing;

pub enum Gate {
    And2(And2),
    And3(And3),
    Inv(Inv),
    FoldedInv(FoldedInv),
    Nand2(Nand2),
    Nand3(Nand3),
    Nor2(Nor2),
}

pub struct TappedGate {
    params: GateParams,
}

pub struct And2 {
    params: AndParams,
}

pub struct And3 {
    params: AndParams,
}

pub struct Inv {
    params: PrimitiveGateParams,
}

pub struct FoldedInv {
    params: PrimitiveGateParams,
}

pub struct Nand2 {
    params: PrimitiveGateParams,
}

pub struct Nand3 {
    params: PrimitiveGateParams,
}

pub struct Nor2 {
    params: PrimitiveGateParams,
}

pub struct GateTree {
    params: GateTreeParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct AndParams {
    pub nand: PrimitiveGateParams,
    pub inv: PrimitiveGateParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum GateParams {
    And2(AndParams),
    And3(AndParams),
    Inv(PrimitiveGateParams),
    FoldedInv(PrimitiveGateParams),
    Nand2(PrimitiveGateParams),
    Nand3(PrimitiveGateParams),
    Nor2(PrimitiveGateParams),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum GateType {
    And2,
    And3,
    Inv,
    FoldedInv,
    Nand2,
    Nand3,
    Nor2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum PrimitiveGateType {
    Inv,
    FoldedInv,
    Nand2,
    Nand3,
    Nor2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct PrimitiveGateParams {
    pub nwidth: i64,
    pub pwidth: i64,
    pub length: i64,
}

pub struct GateTreeParams {
    max_width: i64,
    dir: subgeom::Dir,
    nodes: Vec<GateTreeNodeParams>,
    interstage_buses: Vec<Vec<ArcStr>>,
}

pub struct GateTreeNodeParams {
    max_width: i64,
    width: i64,
    tap_width: i64,
    tap_period: usize,
    dir: subgeom::Dir,
    line: i64,
    space: i64,
    params: GateParams,
    invs: Vec<PrimitiveGateParams>,
    conns: Vec<HashMap<ArcStr, ArcStr>>,
}

impl GateType {
    pub fn primitive_gates(&self) -> Vec<GateType> {
        match *self {
            GateType::And2 => vec![GateType::Nand2, GateType::Inv],
            GateType::And3 => vec![GateType::Nand3, GateType::Inv],
            GateType::Inv => vec![GateType::Inv],
            GateType::FoldedInv => vec![GateType::FoldedInv],
            GateType::Nand2 => vec![GateType::Nand2],
            GateType::Nand3 => vec![GateType::Nand3],
            GateType::Nor2 => vec![GateType::Nor2],
        }
    }

    pub fn is_inv(&self) -> bool {
        matches!(self, GateType::Inv | GateType::FoldedInv)
    }

    pub fn is_and(&self) -> bool {
        matches!(self, GateType::And2 | GateType::And3)
    }

    pub fn is_nand(&self) -> bool {
        matches!(self, GateType::Nand2 | GateType::Nand3)
    }
}

impl PrimitiveGateParams {
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            nwidth: ((self.nwidth as f64) * factor).round() as i64,
            pwidth: ((self.pwidth as f64) * factor).round() as i64,
            length: self.length,
        }
    }
}

impl AndParams {
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            nand: self.nand.scale(factor),
            inv: self.inv.scale(factor),
        }
    }
}

impl GateParams {
    pub fn new_primitive(gt: GateType, params: PrimitiveGateParams) -> Self {
        match gt {
            GateType::Inv => Self::Inv(params),
            GateType::FoldedInv => Self::FoldedInv(params),
            GateType::Nand2 => Self::Nand2(params),
            GateType::Nand3 => Self::Nand3(params),
            GateType::Nor2 => Self::Nor2(params),
            _ => panic!("not a primitive gate"),
        }
    }

    pub fn new_and(gt: GateType, params: AndParams) -> Self {
        match gt {
            GateType::And2 => Self::And2(params),
            GateType::And3 => Self::And3(params),
            _ => panic!("not an and gate"),
        }
    }

    pub fn num_inputs(&self) -> usize {
        match self {
            GateParams::And2(_) => 2,
            GateParams::And3(_) => 3,
            GateParams::Inv(_) => 1,
            GateParams::FoldedInv(_) => 1,
            GateParams::Nand2(_) => 2,
            GateParams::Nand3(_) => 3,
            GateParams::Nor2(_) => 2,
        }
    }

    pub fn scale(&self, factor: f64) -> Self {
        match self {
            GateParams::And2(x) => Self::And2(x.scale(factor)),
            GateParams::And3(x) => Self::And3(x.scale(factor)),
            GateParams::Inv(x) => Self::Inv(x.scale(factor)),
            GateParams::FoldedInv(x) => Self::FoldedInv(x.scale(factor)),
            GateParams::Nand2(x) => Self::Nand2(x.scale(factor)),
            GateParams::Nand3(x) => Self::Nand3(x.scale(factor)),
            GateParams::Nor2(x) => Self::Nor2(x.scale(factor)),
        }
    }

    pub fn gate_type(&self) -> GateType {
        match self {
            GateParams::And2(_) => GateType::And2,
            GateParams::And3(_) => GateType::And3,
            GateParams::Inv(_) => GateType::Inv,
            GateParams::FoldedInv(_) => GateType::FoldedInv,
            GateParams::Nand2(_) => GateType::Nand2,
            GateParams::Nand3(_) => GateType::Nand3,
            GateParams::Nor2(_) => GateType::Nor2,
        }
    }

    pub fn first_gate_sizing(&self) -> PrimitiveGateParams {
        match self {
            GateParams::And2(a) => a.nand,
            GateParams::And3(a) => a.nand,
            GateParams::Inv(x) => *x,
            GateParams::FoldedInv(x) => *x,
            GateParams::Nand2(x) => *x,
            GateParams::Nand3(x) => *x,
            GateParams::Nor2(x) => *x,
        }
    }

    pub fn last_gate_sizing(&self) -> PrimitiveGateParams {
        match self {
            GateParams::And2(a) => a.inv,
            GateParams::And3(a) => a.inv,
            GateParams::Inv(x) => *x,
            GateParams::FoldedInv(x) => *x,
            GateParams::Nand2(x) => *x,
            GateParams::Nand3(x) => *x,
            GateParams::Nor2(x) => *x,
        }
    }

    pub fn primitive_gates(&self) -> Vec<(PrimitiveGateType, PrimitiveGateParams)> {
        match self {
            GateParams::And2(x) => vec![
                (PrimitiveGateType::Nand2, x.nand),
                (PrimitiveGateType::Inv, x.inv),
            ],
            GateParams::And3(x) => vec![
                (PrimitiveGateType::Nand3, x.nand),
                (PrimitiveGateType::Inv, x.inv),
            ],
            GateParams::Inv(x) => vec![(PrimitiveGateType::Inv, *x)],
            GateParams::FoldedInv(x) => vec![(PrimitiveGateType::Inv, *x)],
            GateParams::Nand2(x) => vec![(PrimitiveGateType::Nand2, *x)],
            GateParams::Nand3(x) => vec![(PrimitiveGateType::Nand3, *x)],
            GateParams::Nor2(x) => vec![(PrimitiveGateType::Nor2, *x)],
        }
    }
}

macro_rules! call_gate_fn {
    ($name:expr, $fn_call:ident, $($arg:expr),*) => {
        match $name {
            Gate::And2(gate) => gate.$fn_call($($arg),*),
            Gate::And3(gate) => gate.$fn_call($($arg),*),
            Gate::Inv(gate) => gate.$fn_call($($arg),*),
            Gate::FoldedInv(gate) => gate.$fn_call($($arg),*),
            Gate::Nand2(gate) => gate.$fn_call($($arg),*),
            Gate::Nand3(gate) => gate.$fn_call($($arg),*),
            Gate::Nor2(gate) => gate.$fn_call($($arg),*),
        }
    };
}

impl Component for Gate {
    type Params = GateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(match *params {
            GateParams::And2(params) => Self::And2(And2 { params }),
            GateParams::And3(params) => Self::And3(And3 { params }),
            GateParams::Inv(params) => Self::Inv(Inv { params }),
            GateParams::FoldedInv(params) => Self::FoldedInv(FoldedInv { params }),
            GateParams::Nand2(params) => Self::Nand2(Nand2 { params }),
            GateParams::Nand3(params) => Self::Nand3(Nand3 { params }),
            GateParams::Nor2(params) => Self::Nor2(Nor2 { params }),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        call_gate_fn!(self, name,)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        call_gate_fn!(self, schematic, ctx)
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        call_gate_fn!(self, layout, ctx)
    }
}

impl Component for TappedGate {
    type Params = GateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(TappedGate { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_gate")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let mut gate = ctx.instantiate::<Gate>(&self.params)?;
        ctx.bubble_all_ports(&mut gate);
        ctx.add_instance(gate);
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let li = layers.get(Selector::Metal(0))?;
        let stripe_metal = layers.get(Selector::Metal(1))?;
        let wire_metal = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let decoder_params = DecoderGateParams {
            gate: self.params,
            filler: false,
            dsn: decoder::layout::PhysicalDesign {
                width: 1_580,
                tap_width: 1_580,
                tap_period: 1,
                stripe_metal,
                wire_metal,
                via_metals: vec![],
                li,
                line: 320,
                space: 160,
                rail_width: 320,
                abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
            },
        };
        let gate = ctx.instantiate::<DecoderGate>(&decoder_params)?;
        let tap = ctx.instantiate::<DecoderTap>(&decoder_params)?;
        let mut tiler = ArrayTiler::builder()
            .push(tap)
            .push(gate)
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::CenterVertical)
            .build();
        tiler.expose_ports(|port: CellPort, _i| Some(port), PortConflictStrategy::Merge)?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();

        ctx.draw_ref(&tiler)?;

        Ok(())
    }
}

impl Component for And2 {
    type Params = AndParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("and2")
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

impl Component for And3 {
    type Params = AndParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("and3")
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

impl Component for Inv {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("inv")
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

impl Inv {
    pub fn dec_params() -> PrimitiveGateParams {
        PrimitiveGateParams {
            nwidth: 1_600,
            pwidth: 2_400,
            length: 150,
        }
    }
}

impl Component for FoldedInv {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("folded_inv")
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

impl Component for Nand2 {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("nand2")
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

impl Nand2 {
    pub fn dec_params() -> PrimitiveGateParams {
        PrimitiveGateParams {
            nwidth: 3_200,
            pwidth: 2_400,
            length: 150,
        }
    }
}

impl Component for Nand3 {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("nand3")
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

impl Component for Nor2 {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("nor2")
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

    use std::collections::HashMap;

    use substrate::schematic::netlist::NetlistPurpose;

    use crate::measure::impedance::{
        AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench, TransitionTbNode,
        TransitionTbParams, TransitionTestbench,
    };
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    fn test_inv() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_inv");

        let params = PrimitiveGateParams {
            pwidth: 1_000,
            nwidth: 1_000,
            length: 150,
        };
        ctx.write_layout::<Inv>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<Inv>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let tapped_params = GateParams::Inv(params);
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<TappedGate>(&tapped_params, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<TappedGate>(&tapped_params, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_inv_char() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_inv_char");
        let params = GateParams::Inv(PrimitiveGateParams {
            pwidth: 2_500,
            nwidth: 1_000,
            length: 150,
        });

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<TappedGate>(
            &params,
            &pex_path,
            NetlistPurpose::Pex,
        )
        .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<TappedGate>(&params, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("tapped_gate"),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("tapped_gate"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let pu_zin_work_dir = work_dir.join("pu_zin_sim");
        let pu_zin = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vss,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Floating]),
                    ]),
                },
                &pu_zin_work_dir,
            )
            .expect("failed to write simulation");
        let pu_zout_work_dir = work_dir.join("pu_zout_sim");
        let pu_zout = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Floating,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Vmeas]),
                    ]),
                },
                &pu_zout_work_dir,
            )
            .expect("failed to write simulation");
        println!(
            "Pull-up: Cin = {}, Cout = {}, Rout = {}",
            pu_zin.max_freq_cap(),
            pu_zout.max_freq_cap(),
            pu_zout.min_freq_res()
        );

        let pd_zin_work_dir = work_dir.join("pd_zin_sim");
        let pd_zin = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Floating]),
                    ]),
                },
                &pd_zin_work_dir,
            )
            .expect("failed to write simulation");
        let pd_zout_work_dir = work_dir.join("pd_zout_sim");
        let pd_zout = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Floating,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Vmeas]),
                    ]),
                },
                &pd_zout_work_dir,
            )
            .expect("failed to write simulation");
        println!(
            "Pull-down: Cin = {}, Cout = {}, Rout = {}",
            pd_zin.max_freq_cap(),
            pd_zout.max_freq_cap(),
            pd_zout.min_freq_res()
        );
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_inv_sizing() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_inv_sizing");
        let mut opt = None;
        for pw in (1_000..=4_000).step_by(200) {
            let params = GateParams::Inv(PrimitiveGateParams {
                pwidth: pw,
                nwidth: 1_000,
                length: 150,
            });

            let pex_path = out_spice(&work_dir, "pex_schematic");
            let pex_dir = work_dir.join("pex");
            let pex_level = calibre::pex::PexLevel::Rc;
            let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
            ctx.write_schematic_to_file_for_purpose::<TappedGate>(
                &params,
                &pex_path,
                NetlistPurpose::Pex,
            )
            .expect("failed to write pex source netlist");
            let mut opts = std::collections::HashMap::with_capacity(1);
            opts.insert("level".into(), pex_level.as_str().into());

            let gds_path = out_gds(&work_dir, "layout");
            ctx.write_layout::<TappedGate>(&params, &gds_path)
                .expect("failed to write layout");

            ctx.run_pex(substrate::verification::pex::PexInput {
                work_dir: pex_dir,
                layout_path: gds_path.clone(),
                layout_cell_name: arcstr::literal!("tapped_gate"),
                layout_format: substrate::layout::LayoutFormat::Gds,
                source_paths: vec![pex_path],
                source_cell_name: arcstr::literal!("tapped_gate"),
                pex_netlist_path: pex_netlist_path.clone(),
                ground_net: "vss".to_string(),
                opts,
            })
            .expect("failed to run pex");

            let sim_work_dir = work_dir.join("sim");
            let transitions = ctx
                .write_simulation::<TransitionTestbench<TappedGate>>(
                    &TransitionTbParams {
                        vdd: 1.8,
                        dut: params,
                        delay: 0.1e-9,
                        width: 1e-9,
                        fall: 20e-12,
                        rise: 20e-12,
                        lower_threshold: 0.2,
                        upper_threshold: 0.8,
                        pex_netlist: Some(pex_netlist_path.clone()),
                        connections: HashMap::from_iter([
                            (arcstr::literal!("vdd"), vec![TransitionTbNode::Vdd]),
                            (arcstr::literal!("vss"), vec![TransitionTbNode::Vss]),
                            (arcstr::literal!("a"), vec![TransitionTbNode::Vstim]),
                            (arcstr::literal!("y"), vec![TransitionTbNode::Vmeas]),
                        ]),
                    },
                    &sim_work_dir,
                )
                .expect("failed to write simulation");
            println!(
                "params = {:?}, tr = {:.3}ps, tf={:.3}ps",
                params,
                transitions.tr * 1e12,
                transitions.tf * 1e12
            );
            let diff = (transitions.tr - transitions.tf).abs();
            if let Some((pdiff, _)) = opt {
                if diff < pdiff {
                    opt = Some((diff, params));
                }
            } else {
                opt = Some((diff, params));
            }
        }
        println!("Best parameters: {:?}", opt.unwrap().1);
    }

    #[test]
    fn test_and2() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_and2");

        let params = AndParams {
            nand: PrimitiveGateParams {
                pwidth: 2_400,
                nwidth: 1_800,
                length: 150,
            },
            inv: PrimitiveGateParams {
                pwidth: 2_400,
                nwidth: 1_800,
                length: 150,
            },
        };
        ctx.write_layout::<And2>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<And2>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");
    }

    #[test]
    fn test_and3() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_and3");

        let params = AndParams {
            nand: PrimitiveGateParams {
                pwidth: 2_400,
                nwidth: 4_000,
                length: 150,
            },
            inv: PrimitiveGateParams {
                pwidth: 2_400,
                nwidth: 1_800,
                length: 150,
            },
        };
        ctx.write_layout::<And3>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<And3>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");
    }

    #[test]
    fn test_inv_dec() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_inv_dec");

        let params = Inv::dec_params();
        ctx.write_layout::<Inv>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<Inv>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");
    }

    #[test]
    fn test_nand2_dec() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_nand2_dec");

        let params = Nand2::dec_params();
        ctx.write_layout::<Nand2>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<Nand2>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_nand2_char() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_nand2_char");
        let params = GateParams::Nand2(PrimitiveGateParams {
            pwidth: 2_500,
            nwidth: 2_000,
            length: 150,
        });

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<TappedGate>(
            &params,
            &pex_path,
            NetlistPurpose::Pex,
        )
        .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<TappedGate>(&params, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("tapped_gate"),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("tapped_gate"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let pu_zin_work_dir = work_dir.join("pu_zin_sim");
        let pu_zin = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vss,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Floating]),
                    ]),
                },
                &pu_zin_work_dir,
            )
            .expect("failed to write simulation");
        let pu_zout_work_dir = work_dir.join("pu_zout_sim");
        let pu_zout = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Floating,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Vmeas]),
                    ]),
                },
                &pu_zout_work_dir,
            )
            .expect("failed to write simulation");
        println!(
            "Pull-up: Cin = {}, Cout = {}, Rout = {}",
            pu_zin.max_freq_cap(),
            pu_zout.max_freq_cap(),
            pu_zout.min_freq_res()
        );

        let pd_zin_work_dir = work_dir.join("pd_zin_sim");
        let pd_zin = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Floating]),
                    ]),
                },
                &pd_zin_work_dir,
            )
            .expect("failed to write simulation");
        let pd_zout_work_dir = work_dir.join("pd_zout_sim");
        let pd_zout = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Floating,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Vmeas]),
                    ]),
                },
                &pd_zout_work_dir,
            )
            .expect("failed to write simulation");
        println!(
            "Pull-down: Cin = {}, Cout = {}, Rout = {}",
            pd_zin.max_freq_cap(),
            pd_zout.max_freq_cap(),
            pd_zout.min_freq_res()
        );
    }

    #[test]
    fn test_nand3() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_nand3");

        let params = PrimitiveGateParams {
            nwidth: 1_600,
            pwidth: 2_400,
            length: 150,
        };
        ctx.write_layout::<Nand3>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<Nand3>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_nand3_char() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_nand3_char");
        let params = GateParams::Nand3(PrimitiveGateParams {
            pwidth: 2_500,
            nwidth: 3_000,
            length: 150,
        });

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<TappedGate>(
            &params,
            &pex_path,
            NetlistPurpose::Pex,
        )
        .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<TappedGate>(&params, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("tapped_gate"),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("tapped_gate"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let pu_zin_work_dir = work_dir.join("pu_zin_sim");
        let pu_zin = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vss,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("c"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Floating]),
                    ]),
                },
                &pu_zin_work_dir,
            )
            .expect("failed to write simulation");
        let pu_zout_work_dir = work_dir.join("pu_zout_sim");
        let pu_zout = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Floating,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("c"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Vmeas]),
                    ]),
                },
                &pu_zout_work_dir,
            )
            .expect("failed to write simulation");
        println!(
            "Pull-up: Cin = {}, Cout = {}, Rout = {}",
            pu_zin.max_freq_cap(),
            pu_zout.max_freq_cap(),
            pu_zout.min_freq_res()
        );

        let pd_zin_work_dir = work_dir.join("pd_zin_sim");
        let pd_zin = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("c"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Floating]),
                    ]),
                },
                &pd_zin_work_dir,
            )
            .expect("failed to write simulation");
        let pd_zout_work_dir = work_dir.join("pd_zout_sim");
        let pd_zout = ctx
            .write_simulation::<AcImpedanceTestbench<TappedGate>>(
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Floating,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("a"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("c"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("y"), vec![AcImpedanceTbNode::Vmeas]),
                    ]),
                },
                &pd_zout_work_dir,
            )
            .expect("failed to write simulation");
        println!(
            "Pull-down: Cin = {}, Cout = {}, Rout = {}",
            pd_zin.max_freq_cap(),
            pd_zout.max_freq_cap(),
            pd_zout.min_freq_res()
        );
    }

    #[test]
    fn test_nor2() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_nor2");

        let params = PrimitiveGateParams {
            nwidth: 1_200,
            pwidth: 3_000,
            length: 150,
        };
        ctx.write_layout::<Nor2>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<Nor2>(&params, out_spice(&work_dir, "netlist"))
            .expect("failed to write schematic");
    }
}
