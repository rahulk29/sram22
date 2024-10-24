use crate::blocks::gate::FoldedInv;
use substrate::error::Result;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::elements::mos::SchematicMos;

use super::DiffBuf;

impl DiffBuf {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let din1 = ctx.port("din1", Direction::Input);
        let din2 = ctx.port("din2", Direction::Input);
        let dout1 = ctx.port("dout1", Direction::Output);
        let dout2 = ctx.port("dout2", Direction::Output);

        if let Some(ref latch) = self.params.latch {
            let [rst, set, q, qb] = ctx.signals(["rst", "set", "q", "qb"]);
            for (din, dout, suffix) in [(&din1, &rst, "1"), (&din2, &set, "2")] {
                let mut buf = ctx.instantiate::<FoldedInv>(&self.params.inv)?;
                buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
                buf.set_name(format!("inbuf_{suffix}"));
                ctx.add_instance(buf);
            }
            let nmos_id = ctx
                .mos_db()
                .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
                .id();
            for (din, dout, suffix) in [(&q, &dout2, "1"), (&qb, &dout1, "2")] {
                let mut buf = ctx.instantiate::<FoldedInv>(&latch.inv_out)?;
                buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
                buf.set_name(format!("outbuf_{suffix}"));
                ctx.add_instance(buf);
            }
            for (din, dout, suffix) in [(&q, &qb, "1"), (&qb, &q, "2")] {
                let mut buf = ctx.instantiate::<FoldedInv>(&latch.invq)?;
                buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
                buf.set_name(format!("invq_{suffix}"));
                ctx.add_instance(buf);
            }

            for (d, g, suffix) in [(&q, &rst, "1"), (&qb, &set, "2")] {
                let mut mn = ctx.instantiate::<SchematicMos>(&MosParams {
                    w: latch.nwidth,
                    l: latch.lch,
                    m: 1,
                    nf: 1,
                    id: nmos_id,
                })?;
                mn.connect_all([("d", d), ("g", g), ("s", &vss), ("b", &vss)]);
                mn.set_name(format!("MN{suffix}"));
                ctx.add_instance(mn);
            }
        } else {
            for (din, dout, suffix) in [(&din1, &dout2, "1"), (&din2, &dout1, "2")] {
                let mut buf = ctx.instantiate::<FoldedInv>(&self.params.inv)?;
                buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
                buf.set_name(format!("buf_{suffix}"));
                ctx.add_instance(buf);
            }
        }

        Ok(())
    }
}
