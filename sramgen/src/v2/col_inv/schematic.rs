use substrate::{
    pdk::mos::{query::Query, spec::MosKind, MosParams},
    schematic::{circuit::Direction, elements::mos::SchematicMos},
};

use super::ColInv;

impl ColInv {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let din = ctx.port("din", Direction::Input);
        let din_b = ctx.port("din_b", Direction::Output);

        let mut inv = ctx.instantiate::<Inv>(&GateParams {
            size: Size {
                nmos_width: self.nwidth,
                pmos_width: self.pwidth,
            },
            length: self.length,
        })?;
        inv.connect_all([
            ("vdd", &vdd),
            ("vss", &vss),
            ("din", &din),
            ("din_b", &din_b),
        ]);
        ctx.add_instance(inv);

        Ok(())
    }
}
