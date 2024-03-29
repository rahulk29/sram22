use substrate::component::Component;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::mos::SchematicMos;

use crate::blocks::gate::PrimitiveGateParams;

pub struct TransmissionGate {
    params: PrimitiveGateParams,
}

impl Component for TransmissionGate {
    type Params = PrimitiveGateParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("transmission_gate")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let [din, en, en_b] = ctx.ports(["din", "en", "en_b"], Direction::Input);
        let dout = ctx.port("dout", Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);

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
        .named("npass")
        .with_connections([("d", din), ("g", en), ("s", dout), ("b", vss)])
        .add_to(ctx);

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?
        .named("ppass")
        .with_connections([("d", dout), ("g", en_b), ("s", din), ("b", vdd)])
        .add_to(ctx);

        Ok(())
    }
}
