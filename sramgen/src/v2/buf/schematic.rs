use substrate::error::Result;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use crate::v2::gate::{Inv, PrimitiveGateParams};

use super::{Buf, BufParams, DiffBuf};

impl Buf {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let length = self.params.lch;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let din = ctx.port("din", Direction::Input);
        let dout = ctx.port("dout", Direction::Output);
        let x = ctx.signal("x");

        let inv_params = &PrimitiveGateParams {
            nwidth: self.params.nw,
            pwidth: self.params.pw,
            length,
        };

        let mut inv1 = ctx.instantiate::<Inv>(&inv_params)?;
        inv1.connect_all([("vdd", &vdd), ("vss", &vss), ("din", &din), ("din_b", &x)]);
        inv1.set_name(format!("inv_1"));
        ctx.add_instance(inv1);

        let mut inv2 = ctx.instantiate::<Inv>(&inv_params)?;
        inv2.connect_all([("vdd", &vdd), ("vss", &vss), ("din", &x), ("din_b", &dout)]);
        inv2.set_name(format!("inv_2"));
        ctx.add_instance(inv2);

        Ok(())
    }
}

impl DiffBuf {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let length = self.params.lch;

        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let din1 = ctx.port("din1", Direction::Input);
        let din2 = ctx.port("din2", Direction::Input);
        let dout1 = ctx.port("dout1", Direction::Output);
        let dout2 = ctx.port("dout2", Direction::Output);
        let x1 = ctx.signal("x1");
        let x2 = ctx.signal("x2");

        for (din, dout, suffix) in [(&din1, &dout1, "1"), (&din2, &dout2, "2")] {
            let mut buf = ctx.instantiate::<Buf>(&BufParams {
                pw: self.params.pw,
                nw: self.params.nw,
                lch: self.params.lch,
            })?;
            buf.connect_all([("vdd", &vdd), ("vss", &vss), ("din", &din), ("dout", &dout)]);
            buf.set_name(format!("buf_{suffix}"));
            ctx.add_instance(buf);
        }

        Ok(())
    }
}
