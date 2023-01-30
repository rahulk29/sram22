use serde::{Deserialize, Serialize};
use substrate::component::Component;

pub mod layout;
pub mod schematic;

pub enum Gate {
    And2(And2),
    Inv(Inv),
    Nand2(Nand2),
    Nand3(Nand3),
    Nor2(Nor2),
}

pub struct And2 {
    params: AndParams,
}

pub struct Inv {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct AndParams {
    pub nand: PrimitiveGateParams,
    pub inv: PrimitiveGateParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum GateParams {
    And2(AndParams),
    Inv(PrimitiveGateParams),
    Nand2(PrimitiveGateParams),
    Nand3(PrimitiveGateParams),
    Nor2(PrimitiveGateParams),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum GateType {
    And2,
    Inv,
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

impl GateParams {
    pub fn new_primitive(gt: GateType, params: PrimitiveGateParams) -> Self {
        match gt {
            GateType::Inv => Self::Inv(params),
            GateType::Nand2 => Self::Nand2(params),
            GateType::Nand3 => Self::Nand3(params),
            GateType::Nor2 => Self::Nor2(params),
            _ => panic!("not a primitive gate"),
        }
    }

    pub fn num_inputs(&self) -> usize {
        match self {
            GateParams::And2(_) => 2,
            GateParams::Inv(_) => 1,
            GateParams::Nand2(_) => 2,
            GateParams::Nand3(_) => 3,
            GateParams::Nor2(_) => 2,
        }
    }
}

impl From<GateParams> for fanout::GateType {
    fn from(x: GateParams) -> Self {
        match x {
            GateParams::Inv(_) => fanout::GateType::INV,
            GateParams::Nand2(_) => fanout::GateType::NAND2,
            GateParams::Nand3(_) => fanout::GateType::NAND3,
            _ => panic!("unsupported gate type for fanout calculations"),
        }
    }
}

impl From<GateType> for fanout::GateType {
    fn from(x: GateType) -> Self {
        match x {
            GateType::Inv => fanout::GateType::INV,
            GateType::Nand2 => fanout::GateType::NAND2,
            GateType::Nand3 => fanout::GateType::NAND3,
            _ => panic!("unsupported gate type for fanout calculations"),
        }
    }
}

macro_rules! call_gate_fn {
    ($name:expr, $fn_call:ident, $($arg:expr),*) => {
        match $name {
            Gate::And2(gate) => gate.$fn_call($($arg),*),
            Gate::Inv(gate) => gate.$fn_call($($arg),*),
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
        Ok(match params.clone() {
            GateParams::And2(params) => Self::And2(And2 { params }),
            GateParams::Inv(params) => Self::Inv(Inv { params }),
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

impl Component for And2 {
    type Params = AndParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
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

impl Component for Inv {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
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

impl Component for Nand2 {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
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
        Ok(Self {
            params: params.clone(),
        })
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
        Ok(Self {
            params: params.clone(),
        })
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

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

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
