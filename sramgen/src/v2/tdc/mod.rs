use grid::Grid;
use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::Dir;
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{Instance, Port, PortConflictStrategy};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::LayerBbox;
use substrate::layout::routing::manual::jog::SJog;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;

use super::coarse_tdc::TappedRegister;
use super::decoder::layout::{DecoderGateParams, DecoderTap, PredecoderPhysicalDesignScript};
use super::gate::{GateParams, Inv, PrimitiveGateParams};

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

pub struct TdcCell {
    params: PrimitiveGateParams,
}

impl Component for TdcCell {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tdc_cell")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let inv = ctx.instantiate::<Inv>(&self.params)?;
        let mut ffs = ctx.instantiate::<TappedRegister4>(&NoParams)?;

        let inv0 = inv.with_orientation(Named::R90Cw);
        ctx.draw_ref(&inv0)?;

        let hspace = 400;
        let vspace = 600;
        let mut inv1 = inv0.clone().with_orientation(Named::R90Cw);
        inv1.align_to_the_right_of(inv0.bbox(), hspace);
        inv1.align_top(inv0.bbox());
        ctx.draw_ref(&inv1)?;

        let _cx = inv1.bbox().into_rect().right();

        let mut s11 = inv1.clone().with_orientation(Named::R90Cw);
        s11.align_to_the_right_of(inv1.bbox(), hspace);
        s11.align_top(inv1.bbox());
        ctx.draw_ref(&s11)?;

        let mut s12 = s11.clone();
        s12.align_to_the_right_of(s11.bbox(), hspace);
        ctx.draw_ref(&s12)?;
        let mut s13 = s11.clone();
        s13.align_to_the_right_of(s12.bbox(), hspace);
        ctx.draw_ref(&s13)?;
        let mut s14 = s11.clone();
        s14.align_to_the_right_of(s13.bbox(), hspace);
        ctx.draw_ref(&s14)?;

        let mut s21 = s11.clone();
        s21.align_beneath(s11.bbox(), vspace);
        ctx.draw_ref(&s21)?;

        let mut s22 = s13.clone();
        s22.align_beneath(s13.bbox(), vspace);
        ctx.draw_ref(&s22)?;

        let mut s31 = inv0.clone();
        s31.align_beneath(s21.bbox(), vspace);
        ctx.draw_ref(&s31)?;

        let mut prev = s31.clone();

        let [s32, s33, s34, s35, s36, s37, s38] = [1, 2, 3, 4, 5, 6, 7].map(|_| {
            let mut s3i = prev.clone();
            s3i.align_to_the_right_of(prev.bbox(), hspace);
            ctx.draw_ref(&s3i).expect("failed to draw instance");
            prev = s3i.clone();
            s3i
        });

        let mut s41 = s32.clone();
        s41.align_beneath(s32.bbox(), vspace);
        ctx.draw_ref(&s41)?;
        let mut s42 = s34.clone();
        s42.align_beneath(s32.bbox(), vspace);
        ctx.draw_ref(&s42)?;
        let mut s43 = s36.clone();
        s43.align_beneath(s32.bbox(), vspace);
        ctx.draw_ref(&s43)?;
        let mut s44 = s38.clone();
        s44.align_beneath(s32.bbox(), vspace);
        ctx.draw_ref(&s44)?;

        let dsn = ctx
            .inner()
            .run_script::<PredecoderPhysicalDesignScript>(&NoParams)?
            .as_ref()
            .clone();
        let decoder_gate = DecoderGateParams {
            gate: GateParams::Inv(self.params),
            dsn,
        };

        let mut tap = ctx.instantiate::<DecoderTap>(&decoder_gate)?;
        tap.orientation_mut().reflect_vert();

        let mut tap1l = tap.clone();
        tap1l.align_top(inv0.bbox());
        tap1l.align_to_the_left_of(inv0.bbox(), hspace);
        ctx.draw(tap1l)?;
        let mut tap1r = tap.clone();
        tap1r.align_to_the_right_of(s14.bbox(), hspace);
        ctx.draw(tap1r)?;

        let mut tap2l = tap.clone();
        tap2l.align_top(s21.bbox());
        tap2l.align_to_the_left_of(s21.bbox(), hspace);
        ctx.draw_ref(&tap2l)?;
        let mut tap2r = tap2l.clone();
        tap2r.align_to_the_right_of(s22.bbox(), hspace);
        ctx.draw(tap2r)?;

        let mut tap3l = tap.clone();
        tap3l.align_top(s31.bbox());
        tap3l.align_to_the_left_of(s31.bbox(), hspace);
        ctx.draw_ref(&tap3l)?;
        let mut tap3r = tap3l.clone();
        tap3r.align_to_the_right_of(s38.bbox(), hspace);
        ctx.draw(tap3r)?;

        let mut tap4l = tap.clone();
        tap4l.align_top(s41.bbox());
        tap4l.align_to_the_left_of(s41.bbox(), hspace);
        ctx.draw_ref(&tap4l)?;
        let mut tap4r = tap4l.clone();
        tap4r.align_to_the_right_of(s44.bbox(), hspace);
        ctx.draw(tap4r)?;

        ffs.align_beneath(s41.bbox(), vspace);
        ctx.draw(ffs)?;

        let out0 = inv0.port("y")?;
        let in1 = inv1.port("a")?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Name("li1"))?;

        let sjog = SJog::builder()
            .src(out0.largest_rect(m0)?)
            .dst(in1.largest_rect(m0)?)
            .dir(subgeom::Dir::Horiz)
            .layer(m0)
            .width(200)
            .l1(400)
            .grid(5)
            .build()
            .unwrap();
        ctx.draw(sjog)?;
        let sjog = SJog::builder()
            .src(inv1.port("y")?.largest_rect(m0)?)
            .dst(s11.port("a")?.largest_rect(m0)?)
            .dir(subgeom::Dir::Horiz)
            .layer(m0)
            .width(200)
            .l1(400)
            .grid(5)
            .build()
            .unwrap();
        ctx.draw(sjog)?;
        let sjog = SJog::builder()
            .src(s11.port("y")?.largest_rect(m0)?)
            .dst(s21.port("a")?.largest_rect(m0)?)
            .dir(Dir::Vert)
            .layer(m0)
            .width(200)
            .l1(400)
            .grid(5)
            .build()
            .unwrap();
        ctx.draw(sjog)?;

        let r1 = s11.port("a")?.largest_rect(m0)?;
        let r2 = s14.port("a")?.largest_rect(m0)?;
        ctx.draw_rect(m0, r1.union(r2.bbox()).into_rect());

        let sjog = SJog::builder()
            .src(s12.port("y")?.largest_rect(m0)?)
            .dst(s22.port("a")?.largest_rect(m0)?)
            .dir(Dir::Vert)
            .layer(m0)
            .width(200)
            .l1(400)
            .grid(5)
            .build()
            .unwrap();
        ctx.draw(sjog)?;
        let sjog = SJog::builder()
            .src(s13.port("y")?.largest_rect(m0)?)
            .dst(s22.port("a")?.largest_rect(m0)?)
            .dir(Dir::Vert)
            .layer(m0)
            .width(200)
            .l1(400)
            .grid(5)
            .build()
            .unwrap();
        ctx.draw(sjog)?;

        let mut draw_sjog = |src: &Instance, dst: &Instance| -> substrate::error::Result<()> {
            let sjog = SJog::builder()
                .src(src.port("y")?.largest_rect(m0)?)
                .dst(dst.port("a")?.largest_rect(m0)?)
                .dir(Dir::Vert)
                .layer(m0)
                .width(200)
                .l1(400)
                .grid(5)
                .build()
                .unwrap();
            ctx.draw(sjog)?;
            Ok(())
        };

        draw_sjog(&s21, &s31)?;
        draw_sjog(&s21, &s32)?;
        draw_sjog(&s21, &s33)?;
        draw_sjog(&s21, &s34)?;
        draw_sjog(&s22, &s35)?;
        draw_sjog(&s22, &s36)?;
        draw_sjog(&s22, &s37)?;
        draw_sjog(&s22, &s38)?;

        draw_sjog(&s32, &s41)?;
        draw_sjog(&s33, &s41)?;
        draw_sjog(&s34, &s42)?;
        draw_sjog(&s35, &s42)?;
        draw_sjog(&s36, &s43)?;
        draw_sjog(&s37, &s43)?;
        draw_sjog(&s38, &s44)?;

        Ok(())
    }
}

pub struct TappedRegister4;

impl Component for TappedRegister4 {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_register_4")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let reg = ctx.instantiate::<TappedRegister>(&NoParams)?;
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let reg_a = LayerBbox::new(reg.clone(), outline);
        let reg_b = LayerBbox::new(reg.with_orientation(Named::ReflectVert), outline);

        let mut grid = Grid::new(0, 0);
        grid.push_row(into_vec![reg_a.clone()]);
        grid.push_row(into_vec![reg_b.clone()]);
        grid.push_row(into_vec![reg_a.clone()]);
        grid.push_row(into_vec![reg_b.clone()]);
        let tiler = GridTiler::new(grid);
        ctx.draw(tiler)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::{out_gds, out_spice};
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
    fn test_tdc_cell() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tdc_cell");
        ctx.write_layout::<TdcCell>(&INV_SIZING, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

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
