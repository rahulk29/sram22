use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::mos::SchematicMos;

use super::WriteMux;

impl WriteMux {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.sizing.length;

        let we = ctx.port("we", Direction::Input);
        let wmask = ctx.port("wmask", Direction::Input);
        let data = ctx.port("data", Direction::Input);
        let data_b = ctx.port("data_b", Direction::Input);
        let bl = ctx.port("bl", Direction::InOut);
        let br = ctx.port("br", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let x = ctx.signal("x");
        let y = ctx.signal("y");

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut mmuxbr = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.sizing.mux_width,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mmuxbr.connect_all([("d", &br), ("g", &data), ("s", &x), ("b", &vss)]);
        mmuxbr.set_name("MMUXBR");
        ctx.add_instance(mmuxbr);

        let mut mmuxbl = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.sizing.mux_width,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mmuxbl.connect_all([("d", &bl), ("g", &data_b), ("s", &x), ("b", &vss)]);
        mmuxbl.set_name("MMUXBL");
        ctx.add_instance(mmuxbl);

        let mut mwmask = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.sizing.mux_width,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mwmask.connect_all([("d", &x), ("g", &wmask), ("s", &y), ("b", &vss)]);
        mwmask.set_name("MWMASK");
        ctx.add_instance(mwmask);

        let mut mpd = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.sizing.mux_width,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mpd.connect_all([("d", &y), ("g", &we), ("s", &vss), ("b", &vss)]);
        mpd.set_name("MPD");
        ctx.add_instance(mpd);

        Ok(())
    }
}
