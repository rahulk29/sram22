use substrate::{
    error::Result,
    pdk::mos::{query::Query, spec::MosKind, MosParams},
    schematic::{circuit::Direction, context::SchematicCtx, elements::mos::SchematicMos},
};

use super::DiffBuf;

impl DiffBuf {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let length = self.params.lch;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let din1 = ctx.port("din1", Direction::Input);
        let din2 = ctx.port("din2", Direction::Input);
        let dout1 = ctx.port("dout1", Direction::Output);
        let dout2 = ctx.port("dout2", Direction::Output);
        let x1 = ctx.signal("x1");
        let x2 = ctx.signal("x2");

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        for (din, x, dout, suffix) in [(&din1, &x1, &dout1, "1"), (&din2, &x2, &dout2, "2")] {
            let mut mp1 = ctx.instantiate::<SchematicMos>(&MosParams {
                w: self.params.pw,
                l: length,
                m: 1,
                nf: 1,
                id: pmos_id,
            })?;
            mp1.connect_all([("d", x), ("g", din), ("s", &vdd), ("b", &vdd)]);
            mp1.set_name(format!("MP1{suffix}"));
            ctx.add_instance(mp1);

            let mut mn1 = ctx.instantiate::<SchematicMos>(&MosParams {
                w: self.params.nw,
                l: length,
                m: 1,
                nf: 1,
                id: nmos_id,
            })?;
            mn1.connect_all([("d", x), ("g", din), ("s", &vss), ("b", &vss)]);
            mn1.set_name(format!("MN1{suffix}"));
            ctx.add_instance(mn1);

            let mut mp2 = ctx.instantiate::<SchematicMos>(&MosParams {
                w: self.params.pw,
                l: length,
                m: 1,
                nf: 1,
                id: pmos_id,
            })?;
            mp2.connect_all([("d", dout), ("g", x), ("s", &vdd), ("b", &vdd)]);
            mp2.set_name(format!("MP2{suffix}"));
            ctx.add_instance(mp2);

            let mut mn2 = ctx.instantiate::<SchematicMos>(&MosParams {
                w: self.params.nw,
                l: length,
                m: 1,
                nf: 1,
                id: nmos_id,
            })?;
            mn2.connect_all([("d", dout), ("g", x), ("s", &vss), ("b", &vss)]);
            mn2.set_name(format!("MN2{suffix}"));
            ctx.add_instance(mn2);
        }

        Ok(())
    }
}
