use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::mos::SchematicMos;

use super::{And2, And3, Inv, Nand2, Nand3, Nor2};

impl And2 {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vdd = ctx.port("vdd", Direction::InOut);
        let a = ctx.port("a", Direction::Input);
        let b = ctx.port("b", Direction::Input);
        let y = ctx.port("y", Direction::Output);
        let yb = ctx.port("yb", Direction::Output);
        let vss = ctx.port("vss", Direction::InOut);

        let mut nand = ctx.instantiate::<Nand2>(&self.params.nand)?;
        nand.connect_all([
            ("vdd", &vdd),
            ("a", &a),
            ("b", &b),
            ("y", &yb),
            ("vss", &vss),
        ]);
        ctx.add_instance(nand);

        let mut inv = ctx.instantiate::<Inv>(&self.params.inv)?;
        inv.connect_all([("vdd", &vdd), ("din", &yb), ("din_b", &y), ("vss", &vss)]);
        ctx.add_instance(inv);

        Ok(())
    }
}

impl And3 {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vdd = ctx.port("vdd", Direction::InOut);
        let a = ctx.port("a", Direction::Input);
        let b = ctx.port("b", Direction::Input);
        let c = ctx.port("c", Direction::Input);
        let y = ctx.port("y", Direction::Output);
        let yb = ctx.port("yb", Direction::Output);
        let vss = ctx.port("vss", Direction::InOut);

        let mut nand = ctx.instantiate::<Nand3>(&self.params.nand)?;
        nand.connect_all([
            ("vdd", &vdd),
            ("a", &a),
            ("b", &b),
            ("c", &c),
            ("y", &yb),
            ("vss", &vss),
        ]);
        ctx.add_instance(nand);

        let mut inv = ctx.instantiate::<Inv>(&self.params.inv)?;
        inv.connect_all([("vdd", &vdd), ("din", &yb), ("din_b", &y), ("vss", &vss)]);
        ctx.add_instance(inv);

        Ok(())
    }
}

impl Inv {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let din = ctx.port("din", Direction::Input);
        let din_b = ctx.port("din_b", Direction::Output);

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut mp = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        mp.connect_all([("d", &din_b), ("g", &din), ("s", &vdd), ("b", &vdd)]);
        mp.set_name("MP0");
        ctx.add_instance(mp);

        let mut mn = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mn.connect_all([("d", &din_b), ("g", &din), ("s", &vss), ("b", &vss)]);
        mn.set_name("MN0");
        ctx.add_instance(mn);

        Ok(())
    }
}

impl Nand2 {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let a = ctx.port("a", Direction::Input);
        let b = ctx.port("b", Direction::Input);
        let y = ctx.port("y", Direction::Output);
        let x = ctx.signal("x");

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut n1 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        n1.connect_all([("d", &x), ("g", &a), ("s", &vss), ("b", &vss)]);
        n1.set_name("n1");
        ctx.add_instance(n1);

        let mut n2 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        n2.connect_all([("d", &y), ("g", &b), ("s", &x), ("b", &vss)]);
        n2.set_name("n2");
        ctx.add_instance(n2);

        let mut p1 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        p1.connect_all([("d", &y), ("g", &a), ("s", &vdd), ("b", &vdd)]);
        p1.set_name("p1");
        ctx.add_instance(p1);

        let mut p2 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        p2.connect_all([("d", &y), ("g", &b), ("s", &vdd), ("b", &vdd)]);
        p2.set_name("p2");
        ctx.add_instance(p2);

        Ok(())
    }
}

impl Nand3 {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let a = ctx.port("a", Direction::Input);
        let b = ctx.port("b", Direction::Input);
        let c = ctx.port("c", Direction::Input);
        let y = ctx.port("y", Direction::Output);
        let x1 = ctx.signal("x1");
        let x2 = ctx.signal("x2");

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut n1 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        n1.connect_all([("d", &x1), ("g", &a), ("s", &vss), ("b", &vss)]);
        n1.set_name("n1");
        ctx.add_instance(n1);

        let mut n2 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        n2.connect_all([("d", &x2), ("g", &b), ("s", &x1), ("b", &vss)]);
        n2.set_name("n2");
        ctx.add_instance(n2);

        let mut n3 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        n3.connect_all([("d", &y), ("g", &c), ("s", &x2), ("b", &vss)]);
        n3.set_name("n3");
        ctx.add_instance(n3);

        let mut p1 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        p1.connect_all([("d", &y), ("g", &a), ("s", &vdd), ("b", &vdd)]);
        p1.set_name("p1");
        ctx.add_instance(p1);

        let mut p2 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        p2.connect_all([("d", &y), ("g", &b), ("s", &vdd), ("b", &vdd)]);
        p2.set_name("p2");
        ctx.add_instance(p2);

        let mut p3 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        p3.connect_all([("d", &y), ("g", &c), ("s", &vdd), ("b", &vdd)]);
        p3.set_name("p3");
        ctx.add_instance(p3);

        Ok(())
    }
}

impl Nor2 {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let a = ctx.port("a", Direction::Input);
        let b = ctx.port("b", Direction::Input);
        let y = ctx.port("y", Direction::Output);
        let x = ctx.signal("x");

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut n1 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        n1.connect_all([("d", &y), ("g", &a), ("s", &vss), ("b", &vss)]);
        n1.set_name("n1");
        ctx.add_instance(n1);

        let mut n2 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        n2.connect_all([("d", &y), ("g", &b), ("s", &vss), ("b", &vss)]);
        n2.set_name("n2");
        ctx.add_instance(n2);

        let mut p1 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        p1.connect_all([("d", &y), ("g", &a), ("s", &x), ("b", &vdd)]);
        p1.set_name("p1");
        ctx.add_instance(p1);

        let mut p2 = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        p2.connect_all([("d", &x), ("g", &b), ("s", &vdd), ("b", &vdd)]);
        p2.set_name("p2");
        ctx.add_instance(p2);

        Ok(())
    }
}
