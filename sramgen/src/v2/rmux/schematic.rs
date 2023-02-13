use substrate::{
    pdk::mos::{query::Query, spec::MosKind, MosParams},
    schematic::{circuit::Direction, elements::mos::SchematicMos},
};

use super::ReadMux;

impl ReadMux {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let sel_b = ctx.port("sel_b", Direction::Input);
        let bl = ctx.port("bl", Direction::InOut);
        let br = ctx.port("br", Direction::InOut);
        let bl_out = ctx.port("bl_out", Direction::InOut);
        let br_out = ctx.port("br_out", Direction::InOut);
        let vdd = ctx.port("vdd", Direction::InOut);

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        let mut mbl = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.width,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        mbl.connect_all([("d", &bl_out), ("g", &sel_b), ("s", &bl), ("b", &vdd)]);
        mbl.set_name("MBL");
        ctx.add_instance(mbl);

        let mut mbr = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.width,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        mbr.connect_all([("d", &br_out), ("g", &sel_b), ("s", &br), ("b", &vdd)]);
        mbr.set_name("MBR");
        ctx.add_instance(mbr);

        Ok(())
    }
}
