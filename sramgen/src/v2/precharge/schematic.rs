use substrate::{
    pdk::mos::{query::Query, spec::MosKind, MosParams},
    schematic::{circuit::Direction, elements::mos::SchematicMos},
};

use super::Precharge;

impl Precharge {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let vdd = ctx.port("vdd", Direction::InOut);
        let bl = ctx.port("bl", Direction::InOut);
        let br = ctx.port("br", Direction::InOut);
        let en_b = ctx.port("en_b", Direction::Input);

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        let mut bl_pull_up = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pull_up_width,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        bl_pull_up.connect_all([("d", &bl), ("g", &en_b), ("s", &vdd), ("b", &vdd)]);
        bl_pull_up.set_name("bl_pull_up");
        ctx.add_instance(bl_pull_up);

        let mut br_pull_up = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pull_up_width,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        br_pull_up.connect_all([("d", &br), ("g", &en_b), ("s", &vdd), ("b", &vdd)]);
        br_pull_up.set_name("br_pull_up");
        ctx.add_instance(br_pull_up);

        let mut equalizer = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.equalizer_width,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        equalizer.connect_all([("d", &bl), ("g", &en_b), ("s", &br), ("b", &vdd)]);
        equalizer.set_name("equalizer");
        ctx.add_instance(equalizer);

        Ok(())
    }
}
