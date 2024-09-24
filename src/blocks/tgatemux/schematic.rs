use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::mos::SchematicMos;

use super::*;

impl TGateMux {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let sel_b = ctx.port("sel_b", Direction::Input);
        let sel = ctx.port("sel", Direction::Input);
        let bl = ctx.port("bl", Direction::InOut);
        let br = ctx.port("br", Direction::InOut);
        let bl_out = ctx.port("bl_out", Direction::InOut);
        let br_out = ctx.port("br_out", Direction::InOut);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();
        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut mpbl = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        mpbl.connect_all([("d", &bl_out), ("g", &sel_b), ("s", &bl), ("b", &vdd)]);
        mpbl.set_name("MPBL");
        ctx.add_instance(mpbl);

        let mut mpbr = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        mpbr.connect_all([("d", &br_out), ("g", &sel_b), ("s", &br), ("b", &vdd)]);
        mpbr.set_name("MBR");
        ctx.add_instance(mpbr);

        let mut mnbl = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mnbl.connect_all([("d", &bl_out), ("g", &sel), ("s", &bl), ("b", &vss)]);
        mnbl.set_name("MNBL");
        ctx.add_instance(mnbl);

        let mut mnbr = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mnbr.connect_all([("d", &br_out), ("g", &sel), ("s", &br), ("b", &vss)]);
        mnbr.set_name("MNBR");
        ctx.add_instance(mnbr);

        Ok(())
    }
}
