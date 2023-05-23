use serde::{Deserialize, Serialize};
use substrate::{
    component::Component,
    pdk::mos::{query::Query, spec::MosKind, MosParams},
    schematic::{circuit::Direction, elements::mos::SchematicMos},
};

use crate::v2::gate::{Inv, PrimitiveGateParams};

pub struct TristateInv {
    params: PrimitiveGateParams,
}

pub struct TristateBuf {
    params: TristateBufParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TristateBufParams {
    pub inv1: PrimitiveGateParams,
    pub inv2: PrimitiveGateParams,
}

impl Component for TristateInv {
    type Params = PrimitiveGateParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("tristate_inv")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let [din, en, en_b] = ctx.ports(["din", "en", "en_b"], Direction::Input);
        let din_b = ctx.port("din_b", Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [nint, pint] = ctx.signals(["nint", "pint"]);

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();
        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?
        .named("mn_en")
        .with_connections([("d", din_b), ("g", en), ("s", nint), ("b", vss)])
        .add_to(ctx);

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?
        .named("mn_pd")
        .with_connections([("d", nint), ("g", din), ("s", vss), ("b", vss)])
        .add_to(ctx);

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?
        .named("mp_en")
        .with_connections([("d", din_b), ("g", en_b), ("s", pint), ("b", vdd)])
        .add_to(ctx);

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?
        .named("mp_pu")
        .with_connections([("d", pint), ("g", din), ("s", vdd), ("b", vdd)])
        .add_to(ctx);

        Ok(())
    }
}

impl Component for TristateBuf {
    type Params = TristateBufParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("tristate_buf")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let [din, en, en_b] = ctx.ports(["din", "en", "en_b"], Direction::Input);
        let dout = ctx.port("dout", Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let x = ctx.signal("x");

        ctx.instantiate::<Inv>(&self.params.inv1)?
            .named("inv1")
            .with_connections([("din", din), ("din_b", x), ("vdd", vdd), ("vss", vss)])
            .add_to(ctx);

        ctx.instantiate::<TristateInv>(&self.params.inv2)?
            .named("inv2")
            .with_connections([
                ("din", x),
                ("din_b", dout),
                ("en", en),
                ("en_b", en_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .add_to(ctx);

        Ok(())
    }
}
