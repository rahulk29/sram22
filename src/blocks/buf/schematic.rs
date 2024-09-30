use substrate::error::Result;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use crate::blocks::gate::{FoldedInv, Inv, PrimitiveGateParams};

use super::DiffBuf;

impl DiffBuf {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let _length = self.params.length;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let din1 = ctx.port("din1", Direction::Input);
        let din2 = ctx.port("din2", Direction::Input);
        let dout1 = ctx.port("dout1", Direction::Output);
        let dout2 = ctx.port("dout2", Direction::Output);

        for (din, dout, suffix) in [(&din1, &dout2, "1"), (&din2, &dout1, "2")] {
            let mut buf = ctx.instantiate::<FoldedInv>(&self.params)?;
            buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
            buf.set_name(format!("buf_{suffix}"));
            ctx.add_instance(buf);
        }

        Ok(())
    }
}
