use serde::{Deserialize, Serialize};
use substrate::component::Component;

pub mod layout;
pub mod schematic;

pub enum Gate {
    And2(And2),
    And3(And3),
    Inv(Inv),
    Nand2(Nand2),
    Nand3(Nand3),
    Nor2(Nor2),
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
    And3(AndParams),
    Inv(PrimitiveGateParams),
    Nand2(PrimitiveGateParams),
    Nand3(PrimitiveGateParams),
    Nor2(PrimitiveGateParams),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum GateType {
    And2,
    And3,
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

impl GateType {
    pub fn primitive_gates(&self) -> Vec<GateType> {
        match *self {
            GateType::And2 => vec![GateType::Nand2, GateType::Inv],
            GateType::And3 => vec![GateType::Nand3, GateType::Inv],
            GateType::Inv => vec![GateType::Inv],
            GateType::Nand2 => vec![GateType::Nand2],
            GateType::Nand3 => vec![GateType::Nand3],
            GateType::Nor2 => vec![GateType::Nor2],
        }
    }
}

impl PrimitiveGateParams {
    pub fn scale(&self, factor: i64) -> Self {
        Self {
            nwidth: self.nwidth * factor,
            pwidth: self.pwidth * factor,
            length: self.length,
        }
    }
}

impl AndParams {
    pub fn scale(&self, factor: i64) -> Self {
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
            GateParams::Nand2(_) => 2,
            GateParams::Nand3(_) => 3,
            GateParams::Nor2(_) => 2,
        }
    }

    pub fn scale(&self, factor: i64) -> Self {
        match self {
            GateParams::And2(x) => Self::And2(x.scale(factor)),
            GateParams::And3(x) => Self::And3(x.scale(factor)),
            GateParams::Inv(x) => Self::Inv(x.scale(factor)),
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
            GateParams::Nand2(_) => GateType::Nand2,
            GateParams::Nand3(_) => GateType::Nand3,
            GateParams::Nor2(_) => GateType::Nor2,
        }
    }
}

macro_rules! call_gate_fn {
    ($name:expr, $fn_call:ident, $($arg:expr),*) => {
        match $name {
            Gate::And2(gate) => gate.$fn_call($($arg),*),
            Gate::And3(gate) => gate.$fn_call($($arg),*),
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
        Ok(match *params {
            GateParams::And2(params) => Self::And2(And2 { params }),
            GateParams::And3(params) => Self::And3(And3 { params }),
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
