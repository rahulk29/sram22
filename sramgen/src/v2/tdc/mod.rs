use std::collections::HashSet;

use grid::Grid;
use serde::{Deserialize, Serialize};
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Named;
use subgeom::{Dir, Rect, Side, Sides};
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{CellPort, Instance, Port, PortConflictStrategy, PortId};
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
use substrate::layout::placement::align::AlignRect;

use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::LayerBbox;
use substrate::layout::routing::auto::grid::ExpandToGridStrategy;
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::SJog;
use substrate::layout::routing::tracks::TrackLocator;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;

use super::coarse_tdc::TappedRegister;
use super::decoder::layout::{DecoderGateParams, DecoderTap, PhysicalDesign};
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
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Name("li1"))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let dsn = PhysicalDesign {
            width: 2_000,
            tap_width: 790,
            tap_period: 2,
            stripe_metal: m1,
            wire_metal: m0,
            via_metals: vec![],
            li: m0,
            line: 320,
            space: 160,
            rail_width: 320,
            abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
        };
        let decoder_gate = DecoderGateParams {
            gate: GateParams::Inv(self.params),
            dsn,
        };

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

        let mut tap = ctx.instantiate::<DecoderTap>(&decoder_gate)?;
        tap.orientation_mut().reflect_vert();

        let mut tap1l = tap.clone();
        tap1l.align_top(inv0.bbox());
        tap1l.align_to_the_left_of(inv0.bbox(), hspace);
        ctx.draw_ref(&tap1l)?;
        let mut tap1r = tap.clone();
        tap1r.align_to_the_right_of(s14.bbox(), hspace);
        ctx.draw_ref(&tap1r)?;

        let mut tap2l = tap.clone();
        tap2l.align_top(s21.bbox());
        tap2l.align_to_the_left_of(s21.bbox(), hspace);
        ctx.draw_ref(&tap2l)?;
        let mut tap2r = tap2l.clone();
        tap2r.align_to_the_right_of(s22.bbox(), hspace);
        ctx.draw_ref(&tap2r)?;

        let mut tap3l = tap.clone();
        tap3l.align_top(s31.bbox());
        tap3l.align_to_the_left_of(s31.bbox(), hspace);
        ctx.draw_ref(&tap3l)?;
        let mut tap3r = tap3l.clone();
        tap3r.align_to_the_right_of(s38.bbox(), hspace);
        ctx.draw_ref(&tap3r)?;

        let mut tap4l = tap.clone();
        tap4l.align_top(s41.bbox());
        tap4l.align_to_the_left_of(s41.bbox(), hspace);
        ctx.draw_ref(&tap4l)?;
        let mut tap4r = tap4l.clone();
        tap4r.align_to_the_right_of(s44.bbox(), hspace);
        ctx.draw_ref(&tap4r)?;

        ffs.align_beneath(s41.bbox(), 4 * vspace);
        ctx.draw_ref(&ffs)?;

        let r1 = s11.port("a")?.largest_rect(m0)?;
        let r2 = s14.port("a")?.largest_rect(m0)?;
        ctx.draw_rect(m0, r1.union(r2.bbox()).into_rect());

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

        draw_sjog(&inv0, &inv1)?;
        draw_sjog(&inv1, &s11)?;
        draw_sjog(&s11, &s12)?;
        draw_sjog(&s12, &s22)?;
        draw_sjog(&s13, &s22)?;

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

        let row1 = vec![&tap1l, &inv0, &inv1, &s11, &s12, &s13, &s14, &tap1r];
        let row2 = vec![&tap2l, &s21, &s22, &tap2r];
        let row3 = vec![
            &tap3l, &s31, &s32, &s33, &s34, &s35, &s36, &s37, &s38, &tap3r,
        ];
        let row4 = vec![&tap4l, &s41, &s42, &s43, &s44, &tap4r];

        let nwell = layers.get(Selector::Name("nwell"))?;

        for row in [row1, row2, row3, row4] {
            for layer in [nwell] {
                let mut bbox = Bbox::empty();
                for inst in row.iter() {
                    bbox = bbox.union(inst.layer_bbox(layer));
                }
                ctx.draw_rect(layer, bbox.into_rect());
            }

            for port in ["vdd", "vss"] {
                let bbox = row[0]
                    .port(port)?
                    .largest_rect(m1)?
                    .bbox()
                    .union(row[row.len() - 1].port(port)?.largest_rect(m1)?.bbox());
                let rect = bbox.into_rect();
                ctx.draw_rect(m1, rect);

                for inst in row[1..row.len() - 1].iter() {
                    let target = inst.port(port)?.largest_rect(m0)?;
                    let viap = ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(target, rect)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw_ref(&via)?;
                }
            }
        }

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(8_000),
            layers: vec![
                LayerConfig {
                    line: 320,
                    space: 140,
                    dir: Dir::Vert,
                    layer: m0,
                },
                LayerConfig {
                    line: 320,
                    space: 140,
                    dir: Dir::Horiz,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 140,
                    dir: Dir::Vert,
                    layer: m2,
                },
            ],
        });

        for layer in [m0, m1, m2] {
            for shape in ffs.shapes_on(layer) {
                if let Some(rect) = shape.as_rect() {
                    let rect = rect;
                    ctx.draw_rect(layer, rect);
                    router.block(layer, rect);
                }
            }
        }

        for (idx, inv) in [(0, s44), (1, s43), (2, s42), (3, s41)] {
            let src = ffs.port(PortId::new("d", idx))?.largest_rect(m0)?;
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(src, src)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(src, src)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            let d = via.layer_bbox(m2);
            ctx.draw(via)?;

            let net = format!("d{idx}");

            let d = router.expand_to_grid(d.into_rect(), ExpandToGridStrategy::Minimum);
            router.occupy(m2, d, &net)?;
            ctx.draw_rect(m2, d);

            let y = inv.port("y")?.largest_rect(m0)?;
            let y =
                router.expand_to_grid(y, ExpandToGridStrategy::Corner(subgeom::Corner::LowerRight));
            router.occupy(m0, y, &net)?;
            ctx.draw_rect(m0, y);

            router.occupy(m2, y, &net)?;

            router.route_with_net(ctx, m0, y, m2, d, &net)?;
        }

        let brect = ctx.brect();

        let vtracks = router.track_info(m2).tracks();
        let vstart =
            vtracks.track_with_loc(TrackLocator::EndsBefore, (brect.right() + brect.left()) / 2);
        let htracks = router.track_info(m1).tracks();
        let htrack = htracks
            .index(htracks.track_with_loc(TrackLocator::EndsBefore, ctx.brect().bottom()) - 5);

        let mut output_rects = Vec::with_capacity(4);
        for i in 0..4 {
            let vtrack = vtracks.index(vstart - 2 * (i as i64) + 2);
            output_rects.push(Rect::from_spans(vtrack, htrack));
            ctx.draw_rect(m2, output_rects[i]);
        }

        for i in 0..4 {
            let q = ffs.port(PortId::new("q", i))?.largest_rect(m0)?;
            let viap = ViaParams::builder().layers(m0, m1).geometry(q, q).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            let q = via.layer_bbox(m1);
            ctx.draw(via)?;
            let side = if i % 2 == 0 { Side::Bot } else { Side::Top };
            let q = router.expand_to_grid(q.into_rect(), ExpandToGridStrategy::Side(side));
            ctx.draw_rect(m1, q);
            let net = format!("q{i}");
            router.occupy(m1, q, &net)?;

            let dst = output_rects[4 - i - 1];
            router.occupy(m2, dst, &net)?;
            router.route_with_net(ctx, m1, q, m2, dst, &net)?;

            let port = CellPort::builder()
                .id(PortId::new(arcstr::literal!("q"), i))
                .add(m2, dst)
                .build();
            ctx.add_port(port)?;
        }

        ctx.draw(router)?;

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
        grid.push_row(into_vec![reg_a]);
        grid.push_row(into_vec![reg_b]);
        let mut tiler = GridTiler::new(grid);
        tiler.expose_ports(
            |mut port: CellPort, idx: (usize, usize)| {
                port.set_id(PortId::new(port.name(), idx.0));
                Some(port)
            },
            PortConflictStrategy::Error,
        )?;
        println!("tapped register 4 ports:");
        for port in tiler.ports() {
            println!("tapped register 4 | name = {}", port.name());
        }
        ctx.add_ports(tiler.ports().cloned())?;
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
