use serde::{Deserialize, Serialize};
use substrate::component::Component;
use substrate::index::IndexOwned;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;

use super::gate::{Inv, PrimitiveGateParams};

pub mod tb;

pub struct Tdc {
    params: TdcParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TdcParams {
    stages: usize,
    inv: PrimitiveGateParams,
}

impl TdcParams {
    pub fn bits_out(&self) -> usize {
        4 * (self.stages - 1)
    }
}

impl Component for Tdc {
    type Params = TdcParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        assert!(params.stages >= 3);
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("tdc_{}", self.params.stages)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let bits_out = self.params.bits_out();

        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [a, b, reset_b] = ctx.ports(["a", "b", "reset_b"], Direction::Input);
        let dout = ctx.bus_port("dout", bits_out, Direction::Output);

        let inv = ctx.instantiate::<Inv>(&self.params.inv)?;

        let n = self.params.stages;

        let stage1 = ctx.bus("stage1", n);
        let int1 = ctx.bus("int1", n);
        let stage2 = ctx.bus("stage2", 2 * n - 1);
        let stage3 = ctx.bus("stage3", 2 * n - 1);
        let stage4 = ctx.bus("stage4", bits_out);
        let stage5 = ctx.bus("stage5", bits_out);

        for i in 0..self.params.stages {
            let sin = if i == 0 { a } else { stage1.index(i - 1) };
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", sin),
                    ("din_b", int1.index(i)),
                ])
                .named(arcstr::format!("s1buf_{i}_0"))
                .add_to(ctx);
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", int1.index(i)),
                    ("din_b", stage1.index(i)),
                ])
                .named(arcstr::format!("s1buf_{i}_1"))
                .add_to(ctx);
        }

        for i in 0..stage2.width() {
            let sin0 = stage1.index(i / 2);
            let sin1 = stage1.index((i + 1) / 2);
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", sin0),
                    ("din_b", stage2.index(i)),
                ])
                .named(arcstr::format!("s2_{i}_0"))
                .add_to(ctx);
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", sin1),
                    ("din_b", stage2.index(i)),
                ])
                .named(arcstr::format!("s2_{i}_1"))
                .add_to(ctx);
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", stage2.index(i)),
                    ("din_b", stage3.index(i)),
                ])
                .named(arcstr::format!("s3_{i}"))
                .add_to(ctx);
        }

        let tmp0 = ctx.signal("tmp0");
        let tmp1 = ctx.signal("tmp1");
        let tmp2 = ctx.signal("tmp2");

        inv.clone()
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("din", stage1.index(stage1.width() - 1)),
                ("din_b", tmp0),
            ])
            .named(arcstr::format!("s2_dummy"))
            .add_to(ctx);

        for i in 0..3 {
            let sout = if i < 2 { tmp1 } else { tmp2 };
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", stage3.index(stage3.width() - 1)),
                    ("din_b", sout),
                ])
                .named(arcstr::format!("s4_dummy_{i}"))
                .add_to(ctx);
        }

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells
            .default_lib()
            .expect("no default standard cell library");
        let ff = lib.try_cell_named("sky130_fd_sc_hd__dfrtp_2")?;
        let ff = ctx.instantiate::<StdCell>(&ff.id())?;

        for i in 0..stage4.width() {
            let sin0 = stage3.index(i / 2);
            let sin1 = stage3.index((i + 1) / 2);
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", sin0),
                    ("din_b", stage4.index(i)),
                ])
                .named(arcstr::format!("s4_{i}_0"))
                .add_to(ctx);
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", sin1),
                    ("din_b", stage4.index(i)),
                ])
                .named(arcstr::format!("s4_{i}_1"))
                .add_to(ctx);
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("din", stage4.index(i)),
                    ("din_b", stage5.index(i)),
                ])
                .named(arcstr::format!("s5_{i}"))
                .add_to(ctx);
            ff.clone()
                .with_connections([
                    ("VGND", vss),
                    ("VNB", vss),
                    ("VPB", vdd),
                    ("VPWR", vdd),
                    ("CLK", b),
                    ("RESET_B", reset_b),
                    ("D", stage5.index(i)),
                    ("Q", dout.index(i)),
                ])
                .named(arcstr::format!("ff_{i}"))
                .add_to(ctx);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::out_spice;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::tb::{TdcTb, TdcTbParams};
    use super::*;

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 1_000,
        pwidth: 1_800,
    };

    const TDC_PARAMS: TdcParams = TdcParams {
        stages: 64,
        inv: INV_SIZING,
    };

    const TDC_TB_PARAMS: TdcTbParams = TdcTbParams {
        inner: TDC_PARAMS,
        vdd: 1.8,
        delta_t: 1e-9,
        tr: 20e-12,
        t_stop: 5e-9,
    };

    #[test]
    fn test_tdc() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tdc");
        ctx.write_schematic_to_file::<Tdc>(&TDC_PARAMS, out_spice(&work_dir, "schematic"))
            .expect("failed to write schematic");
        ctx.write_simulation::<TdcTb>(&TDC_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }
}
