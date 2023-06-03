use std::collections::HashSet;

use arcstr::ArcStr;
use grid::Grid;
use serde::{Deserialize, Serialize};
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Named;
use subgeom::{Dir, Point, Rect, Side, Span};
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{
    CellPort, Instance, MustConnect, Port, PortConflictStrategy, PortId,
};
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::DrawRef;

use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::LayerBbox;
use substrate::layout::routing::auto::grid::ExpandToGridStrategy;
use substrate::layout::routing::auto::straps::{RoutedStraps, Target};
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::{ElbowJog, SJog};
use substrate::layout::routing::tracks::TrackLocator;
use substrate::layout::straps::SingleSupplyNet;
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

        let tmp = ctx.bus("tmp", 6);

        for i in 0..2 {
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    (
                        "din",
                        if i == 0 {
                            stage1.index(0)
                        } else {
                            stage1.index(stage1.width() - 1)
                        },
                    ),
                    ("din_b", tmp.index(i)),
                ])
                .named(arcstr::format!("s2_dummy_{i}"))
                .add_to(ctx);
        }

        for i in 0..4 {
            inv.clone()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    (
                        "din",
                        if i == 0 {
                            stage3.index(0)
                        } else {
                            stage3.index(stage3.width() - 1)
                        },
                    ),
                    ("din_b", tmp.index(i + 2)),
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

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        let start_cell = ctx.instantiate::<TdcCell>(&TdcCellParams {
            inv: self.params.inv,
            kind: TdcCellKind::Start,
        })?;
        let middle_cell = ctx.instantiate::<TdcCell>(&TdcCellParams {
            inv: self.params.inv,
            kind: TdcCellKind::Middle,
        })?;
        let end_cell = ctx.instantiate::<TdcCell>(&TdcCellParams {
            inv: self.params.inv,
            kind: TdcCellKind::End,
        })?;

        let mut tiler = ArrayTiler::builder()
            .push(start_cell)
            .push_num(middle_cell, self.params.stages - 2)
            .push(end_cell)
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::Top)
            .build();

        tiler.expose_ports(
            |port: CellPort, i| {
                let idx = port.id().index();
                match port.name().as_str() {
                    "reset_b" | "vdd" | "vss" => Some(port),
                    "clk" => Some(port.named("b")),
                    "q" => Some(
                        port.named("dout")
                            .with_index(if i == 0 { 0 } else { 4 * i - 2 } + idx),
                    ),
                    "buf_in" => {
                        if i == 0 {
                            Some(port.named("a"))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned())?;
        let group = tiler.draw_ref()?;

        let router_bbox = group
            .brect()
            .expand_dir(Dir::Horiz, 2 * 680)
            .expand_side(Side::Top, 2 * 680)
            .snap_to_grid(680);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: router_bbox,
            layers: vec![
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Horiz,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Vert,
                    layer: m2,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Horiz,
                    layer: m3,
                },
            ],
        });

        let clk_rect = tiler
            .port_map()
            .port("b")?
            .largest_rect(m3)?
            .with_hspan(router_bbox.hspan());
        ctx.merge_port(CellPort::with_shape("b", m3, clk_rect));
        router.block(m3, clk_rect);

        let reset_b_rect = tiler
            .port_map()
            .port("reset_b")?
            .largest_rect(m3)?
            .with_hspan(router_bbox.hspan());
        ctx.merge_port(CellPort::with_shape("reset_b", m3, reset_b_rect));
        router.block(m3, reset_b_rect);

        for layer in [m1, m2, m3] {
            for shape in group.shapes_on(layer) {
                router.block(layer, shape.brect());
            }
            for port in tiler.ports() {
                for shape in port.shapes(layer) {
                    router.block(layer, shape.brect());
                }
            }
        }

        let mut straps = RoutedStraps::new();
        straps.set_strap_layers([m2, m3]);

        for (port_name, net) in [("vdd", SingleSupplyNet::Vdd), ("vss", SingleSupplyNet::Vss)] {
            for shape in tiler.port_map().port(port_name)?.shapes(m1) {
                straps.add_target(m1, Target::new(net, shape.brect()));
            }
        }

        straps.fill(&router, ctx)?;

        ctx.draw(group)?;

        Ok(())
    }
}

pub struct TdcCell {
    params: TdcCellParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TdcCellKind {
    Start,
    Middle,
    End,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TdcCellParams {
    inv: PrimitiveGateParams,
    kind: TdcCellKind,
}

impl Component for TdcCell {
    type Params = TdcCellParams;
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
        let m3 = layers.get(Selector::Metal(3))?;
        let via01 = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m0, m1)
                .geometry(
                    Rect::from_point(Point::zero()),
                    Rect::from_point(Point::zero()),
                )
                .build(),
        )?;
        let via12 = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m1, m2)
                .geometry(
                    Rect::from_point(Point::zero()),
                    Rect::from_point(Point::zero()),
                )
                .build(),
        )?;
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
            gate: GateParams::Inv(self.params.inv),
            dsn,
        };

        let inv = ctx.instantiate::<Inv>(&self.params.inv)?;
        let mut ffs = match self.params.kind {
            TdcCellKind::Start | TdcCellKind::End => ctx.instantiate::<TappedRegisterN>(&2)?,
            TdcCellKind::Middle => ctx.instantiate::<TappedRegisterN>(&4)?,
        };
        let mut ffs_placement = ctx.instantiate::<TappedRegisterN>(&4)?;
        let num_outputs = match self.params.kind {
            TdcCellKind::Start | TdcCellKind::End => 2,
            TdcCellKind::Middle => 4,
        };

        let inv0 = inv.with_orientation(Named::R90Cw);
        ctx.draw_ref(&inv0)?;

        let hspace = 400;
        let vspace = 600;
        let mut inv1 = inv0.clone().with_orientation(Named::R90Cw);
        inv1.align_to_the_right_of(inv0.bbox(), hspace);
        inv1.align_top(inv0.bbox());

        let _cx = inv1.bbox().into_rect().right();

        let mut s11 = inv1.clone().with_orientation(Named::R90Cw);
        s11.align_to_the_right_of(inv1.bbox(), hspace);
        s11.align_top(inv1.bbox());

        let mut s12 = s11.clone();
        s12.align_to_the_right_of(s11.bbox(), hspace);
        let mut s13 = s11.clone();
        s13.align_to_the_right_of(s12.bbox(), hspace);
        let mut s14 = s11.clone();
        s14.align_to_the_right_of(s13.bbox(), hspace);

        let mut s21 = s11.clone();
        s21.align_beneath(s11.bbox(), vspace);

        let mut s22 = s13.clone();
        s22.align_beneath(s13.bbox(), vspace);

        let mut s31 = inv0.clone();
        s31.align_beneath(s21.bbox(), vspace);

        ctx.draw_ref(&inv1)?;
        ctx.draw_ref(&s11)?;
        ctx.draw_ref(&s12)?;
        ctx.draw_ref(&s13)?;
        ctx.draw_ref(&s14)?;
        ctx.draw_ref(&s22)?;

        match self.params.kind {
            TdcCellKind::End | TdcCellKind::Middle => {
                ctx.draw_ref(&s21)?;
                ctx.draw_ref(&s31)?;
            }
            TdcCellKind::Start => {}
        }

        let mut prev = s31.clone();

        let [s32, s33, s34, s35, s36, s37, s38] = [1, 2, 3, 4, 5, 6, 7].map(|i| {
            let mut s3i = prev.clone();
            s3i.align_to_the_right_of(prev.bbox(), hspace);
            if match self.params.kind {
                TdcCellKind::Start => i > 3,
                TdcCellKind::Middle | TdcCellKind::End => true,
            } {
                ctx.draw_ref(&s3i).expect("failed to draw instance");
            }
            prev = s3i.clone();
            s3i
        });

        let mut s41 = s32.clone();
        s41.align_beneath(s32.bbox(), vspace);
        let mut s42 = s34.clone();
        s42.align_beneath(s32.bbox(), vspace);
        let mut s43 = s36.clone();
        s43.align_beneath(s32.bbox(), vspace);
        let mut s44 = s38.clone();
        s44.align_beneath(s32.bbox(), vspace);

        match self.params.kind {
            TdcCellKind::Start => {
                ctx.draw_ref(&s43)?;
                ctx.draw_ref(&s44)?;
            }
            TdcCellKind::Middle => {
                ctx.draw_ref(&s41)?;
                ctx.draw_ref(&s42)?;
                ctx.draw_ref(&s43)?;
                ctx.draw_ref(&s44)?;
            }
            TdcCellKind::End => {
                ctx.draw_ref(&s41)?;
                ctx.draw_ref(&s42)?;
            }
        }

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
        ffs_placement.align_beneath(s41.bbox(), 4 * vspace);
        ctx.draw_ref(&ffs)?;

        let brect = ctx
            .bbox()
            .union(ffs_placement.bbox())
            .into_rect()
            .expand_dir(Dir::Horiz, 700);
        let rect = inv0.port("a")?.largest_rect(m0)?;
        let rect = rect.with_hspan(rect.hspan().add_point(brect.left()));
        ctx.add_port(CellPort::with_shape("buf_in", m0, rect))?;

        let r1 = s11.port("a")?.largest_rect(m0)?;
        let rect = r1.with_hspan(r1.hspan().add_point(brect.right()));
        ctx.draw_rect(m0, rect);
        ctx.add_port(CellPort::with_shape("buf_out", m0, rect))?;

        let mut draw_sjog = |src: &Instance, dst: &Instance| -> substrate::error::Result<SJog> {
            let sjog = SJog::builder()
                .src(src.port("y")?.largest_rect(m0)?)
                .dst(dst.port("a")?.largest_rect(m0)?)
                .dir(Dir::Horiz)
                .layer(m0)
                .width(200)
                .l1(400)
                .grid(5)
                .build()
                .unwrap();
            ctx.draw_ref(&sjog)?;
            Ok(sjog)
        };

        draw_sjog(&inv0, &inv1)?;
        draw_sjog(&inv1, &s11)?;
        let mut draw_sjog = |src: &Instance, dst: &Instance| -> substrate::error::Result<SJog> {
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
            ctx.draw_ref(&sjog)?;
            Ok(sjog)
        };

        draw_sjog(&s12, &s22)?;
        draw_sjog(&s13, &s22)?;

        draw_sjog(&s22, &s35)?;
        draw_sjog(&s22, &s36)?;
        draw_sjog(&s22, &s37)?;
        draw_sjog(&s22, &s38)?;

        let s1back = if !matches!(self.params.kind, TdcCellKind::Start) {
            draw_sjog(&s21, &s31)?;
            draw_sjog(&s21, &s32)?;
            draw_sjog(&s21, &s33)?;
            draw_sjog(&s21, &s34)?;
            draw_sjog(&s32, &s41)?;
            draw_sjog(&s33, &s41)?;
            draw_sjog(&s34, &s42)?;
            draw_sjog(&s35, &s42)?;

            let jog = draw_sjog(&s11, &s21)?;
            let mut s1back = jog.r2();
            s1back.p0.x = brect.left();
            Some(s1back)
        } else {
            None
        };

        if !matches!(self.params.kind, TdcCellKind::End) {
            draw_sjog(&s36, &s43)?;
            draw_sjog(&s37, &s43)?;
            let jog = draw_sjog(&s38, &s44)?;
            let mut s3forward = jog.r2();
            s3forward.p1.x = brect.right();
            ctx.draw_rect(m0, s3forward);
            ctx.add_port(CellPort::with_shape("interp2_out", m0, s3forward))?;
        }

        if !matches!(self.params.kind, TdcCellKind::Start) {
            let s1back = s1back.unwrap();
            ctx.draw_rect(m0, s1back);
            ctx.add_port(CellPort::with_shape("interp1_in", m0, s1back))?;

            let interp2_in = s31.port("y")?.largest_rect(m0)?;
            let jog = ElbowJog::builder()
                .src(interp2_in.edge(Side::Bot))
                .dst(Point::new(brect.left(), interp2_in.bottom() - 400))
                .layer(m0)
                .width2(200)
                .build()
                .unwrap();
            ctx.add_port(CellPort::with_shape("interp2_in", m0, jog.r2()))?;
            ctx.draw(jog)?;
        }

        if !matches!(self.params.kind, TdcCellKind::End) {
            let interp2_in = s14.port("y")?.largest_rect(m0)?;
            let jog = ElbowJog::builder()
                .src(interp2_in.edge(Side::Bot))
                .dst(Point::new(brect.right(), interp2_in.bottom() - 400))
                .layer(m0)
                .width2(200)
                .build()
                .unwrap();
            ctx.add_port(CellPort::with_shape("interp1_out", m0, jog.r2()))?;
            ctx.draw(jog)?;
        }

        let row1 = vec![&tap1l, &inv0, &inv1, &s11, &s12, &s13, &s14, &tap1r];
        let (row2, row3) = if matches!(self.params.kind, TdcCellKind::Start) {
            (
                vec![&tap2l, &s22, &tap2r],
                vec![&tap3l, &s35, &s36, &s37, &s38, &tap3r],
            )
        } else {
            (
                vec![&tap2l, &s21, &s22, &tap2r],
                vec![
                    &tap3l, &s31, &s32, &s33, &s34, &s35, &s36, &s37, &s38, &tap3r,
                ],
            )
        };
        let row4 = match self.params.kind {
            TdcCellKind::Start => vec![&tap4l, &s43, &s44, &tap4r],
            TdcCellKind::Middle => vec![&tap4l, &s41, &s42, &s43, &s44, &tap4r],
            TdcCellKind::End => vec![&tap4l, &s41, &s42, &tap4r],
        };

        let nwell = layers.get(Selector::Name("nwell"))?;

        let mut vdd = CellPort::new("vdd");
        let mut vss = CellPort::new("vss");

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

                if port == "vdd" {
                    vdd.add(m1, rect.into());
                } else {
                    vss.add(m1, rect.into());
                }
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

        ctx.add_port(vdd.with_must_connect(MustConnect::Yes))?;
        ctx.add_port(vss.with_must_connect(MustConnect::Yes))?;

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: brect.expand(8_000),
            layers: vec![
                LayerConfig {
                    line: 320,
                    space: 310,
                    dir: Dir::Vert,
                    layer: m0,
                },
                LayerConfig {
                    line: 320,
                    space: 310,
                    dir: Dir::Horiz,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 310,
                    dir: Dir::Vert,
                    layer: m2,
                },
                LayerConfig {
                    line: 330,
                    space: 300,
                    dir: Dir::Horiz,
                    layer: m3,
                },
            ],
        });

        let clk_track = Span::with_stop_and_length(
            ffs.port(PortId::new("clk", 0))?.largest_rect(m0)?.top(),
            330,
        );
        let clk_rect = Rect::from_spans(ctx.brect().hspan(), clk_track);
        router.block(m3, clk_rect);
        ctx.add_port(CellPort::with_shape("clk", m3, clk_rect))?;

        let reset_b_track = Span::with_start_and_length(
            ffs.port(PortId::new("reset_b", 1))?
                .largest_rect(m0)?
                .bottom(),
            330,
        );
        let reset_b_rect = Rect::from_spans(ctx.brect().hspan(), reset_b_track);
        router.block(m3, reset_b_rect);
        ctx.add_port(CellPort::with_shape("reset_b", m3, reset_b_rect))?;

        for port_name in ["clk", "reset_b"] {
            let mut m2_rect: Option<Bbox> = None;
            for i in 0..num_outputs {
                for via in if port_name == "clk" {
                    vec![&via01, &via12]
                } else {
                    vec![&via12]
                } {
                    let mut via = (*via).clone();
                    let port_rect = ffs.port(PortId::new(port_name, i))?.largest_rect(m0)?;
                    if i % 2 == 0 {
                        via.align_top(port_rect);
                    } else {
                        via.align_bottom(port_rect);
                    }
                    via.align_left(port_rect);
                    if let Some(rect) = m2_rect {
                        m2_rect = Some(rect.bbox().union(via.layer_bbox(m2)));
                    } else {
                        m2_rect = Some(via.layer_bbox(m2));
                    }
                    ctx.draw(via)?;
                }
            }
            let m2_rect = m2_rect.unwrap().into_rect();
            ctx.draw_rect(m2, m2_rect);
            router.block(m2, m2_rect);

            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m2, m3)
                    .geometry(
                        m2_rect,
                        match port_name {
                            "clk" => clk_rect,
                            "reset_b" => reset_b_rect,
                            _ => unreachable!(),
                        },
                    )
                    .build(),
            )?;
            ctx.draw(via)?;
        }

        ctx.merge_port(ffs.port("vpwr")?.into_cell_port().named("vdd"));
        ctx.merge_port(ffs.port("vgnd")?.into_cell_port().named("vss"));

        for layer in [m0, m1, m2] {
            for shape in ffs.shapes_on(layer) {
                router.block(layer, shape.brect());
            }
        }

        for (mut idx, inv) in [s44, s43, s42, s41].iter().enumerate() {
            match self.params.kind {
                TdcCellKind::Start => {
                    if idx >= 2 {
                        continue;
                    }
                }
                TdcCellKind::Middle => {}
                TdcCellKind::End => {
                    if idx < 2 {
                        continue;
                    } else {
                        idx -= 2;
                    }
                }
            }

            let src = ffs.port(PortId::new("d", idx))?.largest_rect(m0)?;
            let mut via1 = via01.clone();
            via1.align_centers_gridded(src, ctx.pdk().layout_grid());
            let mut via2 = via12.clone();
            via2.align_centers_gridded(src, ctx.pdk().layout_grid());
            if idx % 2 == 0 {
                via1.align_bottom(src);
                via2.align_bottom(src);
            } else {
                via1.align_top(src);
                via2.align_top(src);
            }
            router.block(m1, via1.layer_bbox(m1).into_rect());
            router.block(m1, via2.layer_bbox(m1).into_rect());
            let d = via2.layer_bbox(m2).into_rect();
            ctx.draw(via1)?;
            ctx.draw(via2)?;

            let net = format!("d{idx}");

            let d = router.expand_to_grid(d, ExpandToGridStrategy::Minimum);
            router.block(m2, d);
            ctx.draw_rect(m2, d);

            let y = inv.port("y")?.largest_rect(m0)?;
            let y =
                router.expand_to_grid(y, ExpandToGridStrategy::Corner(subgeom::Corner::LowerRight));
            router.occupy(m0, y, &net)?;
            ctx.draw_rect(m0, y);

            router.route_with_net(ctx, m0, y, m2, d, &net)?;
        }

        let vtracks = router.track_info(m2).tracks();
        let vstart =
            vtracks.track_with_loc(TrackLocator::EndsBefore, (brect.right() + brect.left()) / 2);
        let htracks = router.track_info(m1).tracks();
        let htrack =
            htracks.index(htracks.track_with_loc(TrackLocator::EndsBefore, brect.bottom()) - 5);

        let mut output_rects = Vec::with_capacity(num_outputs);
        for i in 0..num_outputs {
            let vtrack = vtracks.index(vstart - 2 * (i as i64) + 2);
            output_rects.push(Rect::from_spans(vtrack, htrack));
            ctx.draw_rect(m2, output_rects[i]);
        }

        for i in 0..num_outputs {
            let q = ffs
                .port(PortId::new("q", num_outputs - i - 1))?
                .largest_rect(m0)?;
            let viap = ViaParams::builder().layers(m0, m1).geometry(q, q).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            let q = via.layer_bbox(m1);
            ctx.draw(via)?;
            let side = if i % 2 == 0 { Side::Bot } else { Side::Top };
            let q = router.expand_to_grid(q.into_rect(), ExpandToGridStrategy::Side(side));
            ctx.draw_rect(m1, q);
            let net = format!("q{i}");
            router.occupy(m1, q, &net)?;

            let dst = output_rects[num_outputs - i - 1];
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

pub struct TappedRegisterN(usize);

impl Component for TappedRegisterN {
    type Params = usize;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self(*params))
    }

    fn name(&self) -> arcstr::ArcStr {
        ArcStr::from(format!("tapped_register_{}", self.0))
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

        for i in 0..self.0 {
            grid.push_row(into_vec![if i % 2 == 0 {
                reg_a.clone()
            } else {
                reg_b.clone()
            }]);
        }
        let mut tiler = GridTiler::new(grid);
        tiler.expose_ports(
            |port: CellPort, idx: (usize, usize)| match port.name().as_str() {
                "vpwr" | "vgnd" => Some(port),
                _ => Some(port.with_index(idx.0)),
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned())?;
        ctx.draw(tiler)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use substrate::schematic::netlist::NetlistPurpose;
    use substrate::verification::pex::PexInput;

    use crate::liberate::save_tdc_lib;
    use crate::paths::{out_gds, out_spice, out_verilog, out_lib};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use crate::v2::sram::verilog::save_tdc_verilog;

    use super::*;

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 1_000,
        pwidth: 1_800,
    };

    const TDC_CELL_PARAMS: TdcCellParams = TdcCellParams {
        inv: INV_SIZING,
        kind: TdcCellKind::Middle,
    };

    const TDC_CELL_END_PARAMS: TdcCellParams = TdcCellParams {
        inv: INV_SIZING,
        kind: TdcCellKind::End,
    };

    const TDC_CELL_START_PARAMS: TdcCellParams = TdcCellParams {
        inv: INV_SIZING,
        kind: TdcCellKind::Start,
    };

    const TDC_PARAMS: TdcParams = TdcParams {
        stages: 64,
        inv: INV_SIZING,
    };

    #[cfg(feature = "commercial")]
    const TDC_TB_PARAMS: super::tb::TdcTbParams = super::tb::TdcTbParams {
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
        ctx.write_layout::<TdcCell>(&TDC_CELL_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_tdc_cell_end() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tdc_cell_end");
        ctx.write_layout::<TdcCell>(&TDC_CELL_END_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_tdc_cell_start() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tdc_cell_start");
        ctx.write_layout::<TdcCell>(&TDC_CELL_START_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_tdc() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tdc");
        ctx.write_schematic_to_file::<Tdc>(&TDC_PARAMS, out_spice(&work_dir, "schematic"))
            .expect("failed to write schematic");
        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<Tdc>(&TDC_PARAMS, &gds_path)
            .expect("failed to write layout");
        #[cfg(feature = "commercial")]
        {
            let tdc = ctx.instantiate_layout::<Tdc>(&TDC_PARAMS).unwrap();
            let name = tdc.cell().name();

            let lib_path = out_lib(&work_dir, name);
            save_tdc_lib(
                &lib_path,
                &crate::verilog::TdcParams {
                    module_name: name.to_string(),
                    data_width: TDC_PARAMS.bits_out(),
                },
            )
            .expect("failed to write lib file from template");

            let verilog_path = out_verilog(&work_dir, name);
            save_tdc_verilog(
                &verilog_path,
                &crate::verilog::TdcParams {
                    module_name: name.to_string(),
                    data_width: TDC_PARAMS.bits_out(),
                },
            )
            .expect("failed to write behavioral model");

            let pex_dir = work_dir.join("pex");
            let pex_source_path = out_spice(&pex_dir, "schematic");
            let pex_out_path = out_spice(&pex_dir, "schematic.pex");
            ctx.write_schematic_to_file_for_purpose::<Tdc>(
                &TDC_PARAMS,
                &pex_source_path,
                NetlistPurpose::Pex,
            )
            .expect("failed to write schematic for PEX");

            crate::abs::run_abstract(
                &work_dir,
                name,
                crate::paths::out_lef(&work_dir, name),
                &gds_path,
                &verilog_path,
            )
            .expect("failed to generate abstract");

            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<Tdc>(&TDC_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<Tdc>(&TDC_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));

            ctx.run_pex(PexInput {
                work_dir: pex_dir,
                layout_path: gds_path.clone(),
                layout_cell_name: name.clone(),
                layout_format: substrate::layout::LayoutFormat::Gds,
                source_paths: vec![pex_source_path],
                source_cell_name: name.clone(),
                pex_netlist_path: pex_out_path,
                opts: Default::default(),
            })
            .expect("failed to run PEX");

            ctx.write_simulation::<super::tb::TdcTb>(&TDC_TB_PARAMS, &work_dir)
                .expect("failed to run simulation");
        }
    }
}
