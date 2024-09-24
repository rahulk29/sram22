use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::mos::SchematicMos;

use crate::blocks::gate::{And2, AndParams, PrimitiveGateParams, TristateInv};

use super::WriteDriver;

impl WriteDriver {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let we = ctx.port("we", Direction::Input);
        let wmask = ctx.port("wmask", Direction::Input);
        let data = ctx.port("data", Direction::Input);
        let data_b = ctx.port("data_b", Direction::Input);
        let bl = ctx.port("bl", Direction::InOut);
        let br = ctx.port("br", Direction::InOut);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let [en, en_b] = ctx.signals(["en", "en_b"]);

        ctx.instantiate::<And2>(&AndParams {
            nand: crate::blocks::gate::PrimitiveGateParams {
                nwidth: self.params.nwidth_logic,
                pwidth: self.params.pwidth_logic,
                length: self.params.length,
            },
            inv: crate::blocks::gate::PrimitiveGateParams {
                nwidth: self.params.nwidth_logic,
                pwidth: self.params.pwidth_logic,
                length: self.params.length,
            },
        })?
        .with_connections([
            ("vdd", vdd),
            ("a", we),
            ("b", wmask),
            ("y", en),
            ("yb", en_b),
            ("vss", vss),
        ])
        .named("and_ctl")
        .add_to(ctx);

        ctx.instantiate::<TristateInv>(&PrimitiveGateParams {
            pwidth: self.params.pwidth_driver,
            nwidth: self.params.nwidth_driver,
            length: self.params.length,
        })?
        .with_connections([
            ("vdd", vdd),
            ("a", data_b),
            ("en", en),
            ("enb", en_b),
            ("y", bl),
            ("vss", vss),
        ])
        .named("bldriver")
        .add_to(ctx);

        ctx.instantiate::<TristateInv>(&PrimitiveGateParams {
            pwidth: self.params.pwidth_driver,
            nwidth: self.params.nwidth_driver,
            length: self.params.length,
        })?
        .with_connections([
            ("vdd", vdd),
            ("a", data),
            ("en", en),
            ("enb", en_b),
            ("y", br),
            ("vss", vss),
        ])
        .named("brdriver")
        .add_to(ctx);

        Ok(())
    }
}
