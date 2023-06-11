use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;

use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, PortConflictStrategy};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::tile::LayerBbox;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;

use super::gate::{Inv, PrimitiveGateParams};

pub mod tb;

pub struct CoarseTdc {
    params: CoarseTdcParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoarseTdcParams {
    stages: usize,
    inv: PrimitiveGateParams,
}

impl CoarseTdcParams {
    pub fn bits_out(&self) -> usize {
        self.stages
    }
}

pub struct CoarseTdcCell {
    params: PrimitiveGateParams,
}

impl Component for CoarseTdcCell {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [a, b, reset_b] = ctx.ports(["a", "b", "reset_b"], Direction::Input);
        let [a_out, b_out, d_out] = ctx.ports(["a_out", "b_out", "d_out"], Direction::Output);

        let [a_0, a_1, a_2, b_0] = ctx.signals(["a_0", "a_1", "a_2", "b_0"]);

        let inv = ctx.instantiate::<Inv>(&self.params)?;

        for (din, dout) in [
            (a, a_0),
            (a_0, a_1),
            (a_1, a_2),
            (a_2, a_out),
            (b, b_0),
            (b_0, b_out),
        ] {
            inv.clone()
                .with_connections([("vdd", vdd), ("vss", vss), ("din", din), ("din_b", dout)])
                .named("inv")
                .add_to(ctx);
        }

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells
            .default_lib()
            .expect("no default standard cell library");
        let ff = lib.try_cell_named("sky130_fd_sc_hd__dfrtp_2")?;
        ctx.instantiate::<StdCell>(&ff.id())?
            .with_connections([
                ("VGND", vss),
                ("VNB", vss),
                ("VPB", vdd),
                ("VPWR", vdd),
                ("CLK", b),
                ("RESET_B", reset_b),
                ("D", a),
                ("Q", d_out),
            ])
            .named(arcstr::format!("ff"))
            .add_to(ctx);

        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let vspace = 400;
        let hspace = 400;
        let inv = ctx.instantiate::<Inv>(&self.params)?;
        ctx.draw(inv.clone())?;

        let mut inv2 = inv.clone();
        inv2.align_to_the_right_of(inv.bbox(), hspace);
        ctx.draw_ref(&inv2)?;

        let mut ff = ctx.instantiate::<TappedRegister>(&NoParams)?;
        ff.align_beneath(inv.bbox(), vspace);
        ctx.draw_ref(&ff)?;

        let mut inv3 = inv.clone();
        inv3.align_beneath(ff.bbox(), vspace);
        ctx.draw_ref(&inv3)?;

        let mut inv4 = inv.clone();
        inv4.align_beneath(inv3.bbox(), vspace);
        ctx.draw_ref(&inv4)?;

        let mut inv6 = inv.clone();
        inv6.align_to_the_right_of(inv.bbox(), hspace);
        inv6.align_beneath(ff.bbox(), vspace);
        ctx.draw_ref(&inv6)?;

        let mut inv5 = inv.clone();
        inv5.align_to_the_right_of(inv.bbox(), hspace);
        inv5.align_beneath(inv6.bbox(), vspace);
        ctx.draw_ref(&inv5)?;

        Ok(())
    }
}

pub struct TappedRegister;

impl Component for TappedRegister {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_register")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;

        let tap = lib.try_cell_named("sky130_fd_sc_hd__tap_2")?;
        let tap = ctx.instantiate::<StdCell>(&tap.id())?;
        let tap = LayerBbox::new(tap, outline);
        let ff = lib.try_cell_named("sky130_fd_sc_hd__dfrtp_2")?;
        let ff = ctx.instantiate::<StdCell>(&ff.id())?;
        let ff = LayerBbox::new(ff, outline);

        let mut row = ArrayTiler::builder();
        row.mode(AlignMode::ToTheRight).alt_mode(AlignMode::Top);
        row.push(tap.clone());
        row.push(ff);
        row.push(tap);
        let mut row = row.build();
        row.expose_ports(
            |port: CellPort, i| {
                if i == 1 || port.name() == "vpwr" || port.name() == "vgnd" {
                    Some(port)
                } else {
                    None
                }
            },
            PortConflictStrategy::Merge,
        )?;
        let group = row.generate()?;
        ctx.add_ports(group.ports())?;
        ctx.draw(group)?;

        Ok(())
    }
}

impl Component for CoarseTdc {
    type Params = CoarseTdcParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        assert!(params.stages >= 3);
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("coarse_tdc_{}", self.params.stages)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let n = self.params.bits_out();
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [a, b, reset_b] = ctx.ports(["a", "b", "reset_b"], Direction::Input);
        let dout = ctx.bus_port("dout", n, Direction::Output);

        let a_out = ctx.bus("a_out", n);
        let b_out = ctx.bus("b_out", n);

        for i in 0..n {
            let (asin, bsin) = if i == 0 {
                (a, b)
            } else {
                (a_out.index(i - 1), b_out.index(i - 1))
            };
            ctx.instantiate::<CoarseTdcCell>(&self.params.inv)?
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("a", asin),
                    ("b", bsin),
                    ("reset_b", reset_b),
                    ("a_out", a_out.index(i)),
                    ("b_out", b_out.index(i)),
                    ("d_out", dout.index(i)),
                ])
                .named(arcstr::format!("cell_{i}"))
                .add_to(ctx);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::tb::{CoarseTdcTb, CoarseTdcTbParams};
    use super::*;

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 1_000,
        pwidth: 1_800,
    };

    const TDC_PARAMS: CoarseTdcParams = CoarseTdcParams {
        stages: 64,
        inv: INV_SIZING,
    };

    const TDC_TB_PARAMS: CoarseTdcTbParams = CoarseTdcTbParams {
        inner: TDC_PARAMS,
        vdd: 1.8,
        delta_t: 1e-9,
        tr: 20e-12,
        t_stop: 5e-9,
    };

    #[test]
    fn test_coarse_tdc_cell() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_coarse_tdc_cell");
        ctx.write_schematic_to_file::<CoarseTdcCell>(
            &TDC_PARAMS.inv,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        ctx.write_layout::<CoarseTdcCell>(&TDC_PARAMS.inv, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_layout::<TappedRegister>(&NoParams, out_gds(work_dir, "register"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_coarse_tdc() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_coarse_tdc");
        ctx.write_schematic_to_file::<CoarseTdc>(&TDC_PARAMS, out_spice(&work_dir, "schematic"))
            .expect("failed to write schematic");
        ctx.write_simulation::<CoarseTdcTb>(&TDC_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }

    #[test]
    #[ignore = "slow"]
    fn test_coarse_tdc_sim() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_coarse_tdc_sim");
        ctx.write_simulation::<CoarseTdcTb>(&TDC_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }
}
