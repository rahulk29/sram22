use substrate::schematic::circuit::Direction;

use crate::blocks::delay_line::tristate::TristateInv;
use crate::blocks::gate::{And2, AndParams, PrimitiveGateParams};

use super::WriteDriver;

impl WriteDriver {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let en = ctx.port("en", Direction::Input);
        let en_b = ctx.port("en_b", Direction::Input);
        let data = ctx.port("data", Direction::Input);
        let data_b = ctx.port("data_b", Direction::Input);
        let bl = ctx.port("bl", Direction::InOut);
        let br = ctx.port("br", Direction::InOut);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);

        ctx.instantiate::<TristateInv>(&PrimitiveGateParams {
            pwidth: self.params.pwidth_driver,
            nwidth: self.params.nwidth_driver,
            length: self.params.length,
        })?
        .with_connections([
            ("vdd", vdd),
            ("din", data_b),
            ("en", en),
            ("en_b", en_b),
            ("din_b", bl),
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
            ("din", data),
            ("en", en),
            ("en_b", en_b),
            ("din_b", br),
            ("vss", vss),
        ])
        .named("brdriver")
        .add_to(ctx);

        Ok(())
    }
}
