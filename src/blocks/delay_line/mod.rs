use std::collections::{HashMap, HashSet};

use grid::{grid, Grid};
use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::transform::Translate;
use subgeom::{Point, Rect, Side, Sides, Sign, Span};
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{CellPort, Port, PortConflictStrategy, PortId};
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::{OptionTile, Pad};
use substrate::layout::routing::auto::straps::{RoutedStraps, Target};
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::{ElbowJog, SJog};
use substrate::layout::routing::tracks::UniformTracks;
use substrate::layout::straps::SingleSupplyNet;
use substrate::layout::DrawRef;
use substrate::schematic::circuit::Direction;
use substrate::script::Script;

use self::transmission::TransmissionGate;
use self::tristate::{TristateBuf, TristateBufParams, TristateInv};

use super::decoder::layout::{DecoderGateParams, DecoderTap, PhysicalDesign};
use super::gate::{Inv, PrimitiveGateParams};

pub mod tb;
pub mod transmission;
pub mod tristate;

pub struct NaiveDelayLine {
    params: NaiveDelayLineParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PassGateKind {
    TransmissionGate(PrimitiveGateParams),
    TristateBuf(TristateBufParams),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NaiveDelayLineParams {
    stages: usize,
    inv1: PrimitiveGateParams,
    inv2: PrimitiveGateParams,
    pass: PassGateKind,
}

pub struct TristateInvDelayLine {
    params: TristateInvDelayLineParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TristateInvDelayLineParams {
    stages: usize,
    inv: PrimitiveGateParams,
    tristate_inv: PrimitiveGateParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DelayLineTracks {
    EnBLeft = 0,
    EnLeft = 1,
    Vss = 2,
    DoutMid = 3,
    DoutBot = 4,
    Vdd = 5,
    EnBRight = 6,
    EnRight = 7,
}

impl Component for NaiveDelayLine {
    type Params = NaiveDelayLineParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        assert!(params.stages >= 3);
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("naive_delay_line_{}", self.params.stages)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let clk_in = ctx.port("clk_in", Direction::Input);
        let clk_out = ctx.port("clk_out", Direction::Output);
        let ctl = ctx.bus_port("ctl", self.params.stages, Direction::Input);
        let ctl_b = ctx.bus_port("ctl_b", self.params.stages, Direction::Input);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);

        let clk_int = ctx.bus("clk_int", self.params.stages);
        let buf_int = ctx.bus("buf_int", self.params.stages);

        for i in 0..self.params.stages {
            let stage_in = if i == 0 { clk_in } else { clk_int.index(i - 1) };
            let stage_out = clk_int.index(i);

            for (j, (input, output, params)) in [
                (stage_in, buf_int.index(i), &self.params.inv1),
                (buf_int.index(i), stage_out, &self.params.inv2),
            ]
            .iter()
            .enumerate()
            {
                ctx.instantiate::<Inv>(*params)?
                    .named(format!("buf_inv_{i}_{j}"))
                    .with_connections([
                        ("din", input),
                        ("din_b", output),
                        ("vdd", &vdd),
                        ("vss", &vss),
                    ])
                    .add_to(ctx);
            }

            match self.params.pass {
                PassGateKind::TransmissionGate(params) => {
                    ctx.instantiate::<TransmissionGate>(&params)?
                        .named(format!("pass_{i}"))
                        .with_connections([
                            ("din", stage_out),
                            ("dout", clk_out),
                            ("en", ctl.index(i)),
                            ("en_b", ctl_b.index(i)),
                            ("vdd", vdd),
                            ("vss", vss),
                        ])
                        .add_to(ctx);
                }
                PassGateKind::TristateBuf(params) => {
                    ctx.instantiate::<TristateBuf>(&params)?
                        .named(format!("pass_{i}"))
                        .with_connections([
                            ("din", stage_out),
                            ("dout", clk_out),
                            ("en", ctl.index(i)),
                            ("en_b", ctl_b.index(i)),
                            ("vdd", vdd),
                            ("vss", vss),
                        ])
                        .add_to(ctx);
                }
            }
        }
        Ok(())
    }
}

impl Component for TristateInvDelayLine {
    type Params = TristateInvDelayLineParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        assert!(params.stages >= 3);
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("tristate_inv_delay_line_{}", self.params.stages)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let clk_in = ctx.port("clk_in", Direction::Input);
        let clk_out = ctx.port("clk_out", Direction::Output);
        let ctl = ctx.bus_port("ctl", self.params.stages, Direction::Input);
        let ctl_b = ctx.bus_port("ctl_b", self.params.stages, Direction::Input);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);

        let clk_int_top = ctx.bus("clk_int_top", self.params.stages);
        let clk_int_bot = ctx.bus("clk_int_bot", self.params.stages - 1);

        ctx.instantiate::<Inv>(&self.params.inv)?
            .named("inv_0")
            .with_connections([
                ("din", clk_in),
                ("din_b", clk_int_top.index(0)),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .add_to(ctx);

        ctx.instantiate::<TristateInv>(&self.params.tristate_inv)?
            .named("tristate_inv_mid_0")
            .with_connections([
                ("din", clk_int_top.index(0)),
                ("din_b", clk_out),
                ("en", ctl.index(0)),
                ("en_b", ctl_b.index(0)),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .add_to(ctx);

        for i in 1..self.params.stages {
            ctx.instantiate::<Inv>(&self.params.inv)?
                .named(format!("inv_{i}"))
                .with_connections([
                    ("din", clk_int_top.index(i - 1)),
                    ("din_b", clk_int_top.index(i)),
                    ("vdd", vdd),
                    ("vss", vss),
                ])
                .add_to(ctx);

            ctx.instantiate::<TristateInv>(&self.params.tristate_inv)?
                .named(format!("tristate_inv_mid_{i}"))
                .with_connections([
                    ("din", clk_int_top.index(i)),
                    ("din_b", clk_int_bot.index(i - 1)),
                    ("en", ctl.index(i)),
                    ("en_b", ctl_b.index(i)),
                    ("vdd", vdd),
                    ("vss", vss),
                ])
                .add_to(ctx);

            ctx.instantiate::<TristateInv>(&self.params.tristate_inv)?
                .named(format!("tristate_inv_bot_{i}"))
                .with_connections([
                    ("din", clk_int_bot.index(i - 1)),
                    (
                        "din_b",
                        if i == 1 {
                            clk_out
                        } else {
                            clk_int_bot.index(i - 2)
                        },
                    ),
                    ("en", ctl_b.index(i - 1)),
                    ("en_b", ctl.index(i - 1)),
                    ("vdd", vdd),
                    ("vss", vss),
                ])
                .add_to(ctx);
        }
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;

        let dsn = ctx
            .inner()
            .run_script::<DelayLineTapDesignScript>(&NoParams)?;

        let inv = ctx.instantiate::<Inv>(&self.params.inv)?;
        let mut tstate = ctx.instantiate::<TristateInv>(&self.params.tristate_inv)?;
        let tap = ctx
            .instantiate::<DecoderTap>(&DecoderGateParams {
                gate: super::gate::GateParams::Inv(self.params.inv),
                dsn: (*dsn).clone(),
            })?
            .with_orientation(Named::R90Cw);

        tstate.align_beneath(&inv, 2_000);

        let tstate_padding = 1_600;

        let inv_tile = Pad::new(
            inv.with_orientation(Named::ReflectVert),
            Sides::new(
                0,
                tstate.brect().width() - inv.brect().width() + tstate_padding,
                0,
                0,
            ),
        );

        let tap_tile = Pad::new(
            tap.clone(),
            Sides::new(
                0,
                tstate.brect().width() - tap.brect().width() + tstate_padding,
                0,
                0,
            ),
        );
        let tstate_tile = Pad::new(
            tstate.with_orientation(Named::ReflectVert),
            Sides::new(0, tstate_padding, 0, 0),
        );

        let mut grid: Grid<OptionTile> = grid![
            [tap_tile.clone().into()][inv_tile.clone().into()][tstate_tile.clone().into()]
                [tap_tile.clone().into()][None.into()]
        ];

        for _ in 1..self.params.stages {
            grid.push_col(into_vec![
                tap_tile.clone(),
                inv_tile.clone(),
                tstate_tile.clone(),
                tstate_tile.clone(),
                tap_tile.clone(),
            ]);
        }

        let mut tiler = GridTiler::new(grid);
        tiler.expose_ports(
            |port: CellPort, (i, j)| {
                if i > 0 && i < 4 {
                    let name = format!("{}_{}_{}", port.name(), i - 1, j);
                    Some(port.named(name))
                } else {
                    None
                }
            },
            PortConflictStrategy::Error,
        )?;
        let tgroup = tiler.draw_ref()?;
        for layer in [nsdm, psdm] {
            let mut merge_sdm = HashMap::new();
            for shape in tgroup.shapes_on(layer) {
                merge_sdm
                    .entry(shape.brect().hspan())
                    .or_insert(Vec::new())
                    .push(shape.brect().vspan());
            }

            for (hspan, vspans) in merge_sdm {
                let new_vspans = Span::merge_adjacent(vspans, |a, b| a.min_distance(b) < 300);
                for vspan in new_vspans {
                    ctx.draw_rect(layer, Rect::from_spans(hspan, vspan));
                }
            }
        }
        ctx.draw(tgroup)?;

        for i in 0..self.params.stages {
            let out_port = tiler
                .port_map()
                .port(format!("y_0_{i}"))?
                .largest_rect(m0)?;
            if i < self.params.stages - 1 {
                let in_port_1 = tiler
                    .port_map()
                    .port(format!("a_0_{}", i + 1))?
                    .largest_rect(m0)?;
                let jog = ElbowJog::builder()
                    .src(out_port.edge(Side::Right))
                    .dst(in_port_1.center())
                    .layer(m0)
                    .width2(in_port_1.width())
                    .build()
                    .unwrap();
                ctx.draw(jog)?;
            }
            let in_port_2 = tiler
                .port_map()
                .port(format!("din_1_{}", i))?
                .first_rect(m0, Side::Right)?;
            let jog = ElbowJog::builder()
                .src(out_port.edge(Side::Right))
                .dst(in_port_2.center())
                .layer(m0)
                .width2(in_port_2.width())
                .build()
                .unwrap();
            ctx.draw(jog)?;
        }

        let vtrack_expand = 12_000;

        let mut vtracks = Vec::new();
        for i in 0..self.params.stages {
            let mut cur_vtracks = Vec::new();
            let in_port_left = tiler
                .port_map()
                .port(format!("din_1_{i}"))?
                .first_rect(m0, Side::Left)?;

            let htracks_left = UniformTracks::builder()
                .line(320)
                .space(180)
                .start(in_port_left.right())
                .sign(Sign::Neg)
                .build()
                .unwrap();

            for j in (0..2usize).rev() {
                let rect = Rect::from_spans(htracks_left.index(j), ctx.brect().vspan());
                cur_vtracks.push(rect);
                if i == 0 && j == 1 {
                    continue;
                }
                ctx.draw_rect(m1, rect);
            }
            let vss_strap = Span::from_center_span_gridded(
                tiler
                    .port_map()
                    .port(format!("vss_0_{i}"))?
                    .largest_rect(m0)?
                    .center()
                    .x,
                320,
                ctx.pdk().layout_grid(),
            );

            let vdd_strap = Span::from_center_span_gridded(
                tiler
                    .port_map()
                    .port(format!("vdd_0_{i}"))?
                    .largest_rect(m0)?
                    .center()
                    .x,
                320,
                ctx.pdk().layout_grid(),
            );

            let dout1_strap = Span::from_center_span_gridded(
                vss_strap.center() + (vdd_strap.center() - vss_strap.center()) / 3,
                320,
                ctx.pdk().layout_grid(),
            );

            let dout2_strap = Span::from_center_span_gridded(
                vss_strap.center() + 2 * (vdd_strap.center() - vss_strap.center()) / 3,
                320,
                ctx.pdk().layout_grid(),
            );

            for (draw_rect, strap) in [
                (true, vss_strap),
                (true, dout1_strap),
                (i > 0, dout2_strap),
                (true, vdd_strap),
            ] {
                let rect = Rect::from_spans(strap, ctx.brect().vspan());
                cur_vtracks.push(rect);
                if draw_rect {
                    ctx.draw_rect(m1, rect);
                }
            }

            let in_port_right = tiler
                .port_map()
                .port(format!("din_1_{i}"))?
                .first_rect(m0, Side::Right)?;

            let htracks_right = UniformTracks::builder()
                .line(320)
                .space(180)
                .start(in_port_right.left())
                .sign(Sign::Pos)
                .build()
                .unwrap();

            for j in 0..2usize {
                let rect = Rect::from_spans(htracks_right.index(j), ctx.brect().vspan());
                cur_vtracks.push(rect);
                if i == 0 && j == 1 {
                    continue;
                }
                ctx.draw_rect(m1, rect);
            }

            for (port_name, track) in [("vss", DelayLineTracks::Vss), ("vdd", DelayLineTracks::Vdd)]
            {
                ctx.merge_port(CellPort::with_shape(
                    port_name,
                    m1,
                    cur_vtracks[track as usize],
                ));
                for j in 0..3 {
                    if i == 0 && j == 2 {
                        continue;
                    }
                    for port in tiler
                        .port_map()
                        .port(format!("{port_name}_{j}_{i}"))?
                        .shapes(m0)
                    {
                        let via = ctx.instantiate::<Via>(
                            &ViaParams::builder()
                                .layers(m0, m1)
                                .geometry(port.brect(), cur_vtracks[track as usize])
                                .build(),
                        )?;
                        ctx.draw(via)?;
                    }
                }
            }

            for (port_name, track) in [
                ("dout_1", DelayLineTracks::DoutMid),
                ("dout_2", DelayLineTracks::DoutBot),
                ("en_1", DelayLineTracks::EnLeft),
                ("en_2", DelayLineTracks::EnBLeft),
                ("en_b_1", DelayLineTracks::EnBRight),
                ("en_b_2", DelayLineTracks::EnRight),
            ] {
                if i == 0 && port_name.ends_with('2') {
                    continue;
                }
                let port = tiler
                    .port_map()
                    .port(format!("{}_{}", port_name, i))?
                    .largest_rect(m0)?;
                let port = Rect::from_spans(
                    port.hspan().union(cur_vtracks[track as usize].hspan()),
                    port.vspan(),
                );
                ctx.draw_rect(m0, port);
                let via = ctx.instantiate::<Via>(
                    &ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(port, cur_vtracks[track as usize])
                        .build(),
                )?;
                ctx.draw(via)?;
            }

            ctx.add_port(CellPort::with_shape(
                PortId::new("ctl", i),
                m1,
                cur_vtracks[DelayLineTracks::EnLeft as usize]
                    .expand_dir(subgeom::Dir::Vert, vtrack_expand),
            ))?;
            ctx.add_port(CellPort::with_shape(
                PortId::new("ctl_b", i),
                m1,
                cur_vtracks[DelayLineTracks::EnBRight as usize]
                    .expand_dir(subgeom::Dir::Vert, vtrack_expand),
            ))?;

            if i > 0 {
                let sjog = SJog::builder()
                    .src(
                        tiler
                            .port_map()
                            .port(format!("dout_1_{i}"))?
                            .largest_rect(m0)?,
                    )
                    .dst(
                        tiler
                            .port_map()
                            .port(format!("din_2_{i}"))?
                            .first_rect(m0, Side::Right)?,
                    )
                    .width(170)
                    .l1(170)
                    .dir(subgeom::Dir::Vert)
                    .layer(m0)
                    .build()
                    .unwrap();
                ctx.draw(sjog)?;
            }

            vtracks.push(cur_vtracks);
        }

        let port = tiler.port_map().port("a_0_0")?.largest_rect(m0)?;
        let mut port_vtrack = vtracks[0][0];
        port_vtrack.translate(Point::new(-1_000, 0));
        let port = Rect::from_spans(port.hspan().union(port_vtrack.hspan()), port.vspan());
        ctx.draw_rect(m0, port);
        let via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m0, m1)
                .geometry(port, port_vtrack)
                .build(),
        )?;
        ctx.draw(via)?;
        ctx.add_port(CellPort::with_shape(
            "clk_in",
            m1,
            port_vtrack.expand_dir(subgeom::Dir::Vert, vtrack_expand),
        ))?;
        ctx.add_port(CellPort::with_shape(
            "clk_out",
            m1,
            vtracks[0][DelayLineTracks::DoutMid as usize]
                .expand_dir(subgeom::Dir::Vert, vtrack_expand),
        ))?;

        let router_bbox = ctx
            .brect()
            .expand_dir(subgeom::Dir::Horiz, 2 * 680)
            .expand_dir(subgeom::Dir::Vert, 12 * 680)
            .snap_to_grid(680);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: router_bbox,
            layers: vec![
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: subgeom::Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: subgeom::Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: subgeom::Dir::Vert,
                    layer: m3,
                },
            ],
        });

        let mut straps = RoutedStraps::new();
        straps.set_strap_layers([m2, m3]);

        for tracks in vtracks.iter() {
            straps.add_target(
                m1,
                Target::new(SingleSupplyNet::Vdd, tracks[DelayLineTracks::Vdd as usize]),
            );
            straps.add_target(
                m1,
                Target::new(SingleSupplyNet::Vss, tracks[DelayLineTracks::Vss as usize]),
            );
        }

        let htracks = UniformTracks::builder()
            .line(320)
            .space(180)
            .start(ctx.brect().top() - 500)
            .sign(Sign::Neg)
            .build()
            .unwrap();
        for i in 0..self.params.stages - 1 {
            for (j, (track_a, track_b)) in [
                (DelayLineTracks::EnLeft, DelayLineTracks::EnRight),
                (DelayLineTracks::EnBRight, DelayLineTracks::EnBLeft),
                (DelayLineTracks::DoutMid, DelayLineTracks::DoutBot),
            ]
            .into_iter()
            .enumerate()
            {
                let rect = Rect::from_spans(
                    vtracks[i][track_a as usize]
                        .hspan()
                        .union(vtracks[i + 1][track_b as usize].hspan()),
                    htracks.index(i % 2 + 2 * j),
                );
                ctx.draw_rect(m2, rect);
                router.block(m2, rect);
                for track in [
                    vtracks[i][track_a as usize],
                    vtracks[i + 1][track_b as usize],
                ] {
                    let via = ctx.instantiate::<Via>(
                        &ViaParams::builder()
                            .layers(m1, m2)
                            .geometry(track, rect)
                            .build(),
                    )?;
                    ctx.draw(via)?;
                }
            }
        }

        straps.fill(&router, ctx)?;

        Ok(())
    }
}

pub struct DelayLineTapDesignScript;

impl Script for DelayLineTapDesignScript {
    type Params = NoParams;
    type Output = PhysicalDesign;

    fn run(
        _params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let layers = ctx.layers();
        let li = layers.get(Selector::Metal(0))?;
        let stripe_metal = layers.get(Selector::Metal(1))?;
        let wire_metal = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 1470,
            tap_width: 1470,
            tap_period: 8,
            stripe_metal,
            wire_metal,
            via_metals: vec![],
            li,
            line: 320,
            space: 160,
            rail_width: 320,
            abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
        })
    }
}

#[cfg(test)]
mod tests {
    use substrate::schematic::netlist::NetlistPurpose;
    use substrate::verification::pex::PexInput;

    use crate::blocks::gate::PrimitiveGateParams;
    use crate::blocks::sram::verilog::save_delay_line_verilog;
    #[cfg(feature = "commercial")]
    use crate::liberate::save_delay_line_lib;
    use crate::paths::{out_gds, out_spice, out_verilog};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::tb::{DelayLineTb, DelayLineTbParams};
    use super::tristate::TristateBufParams;
    use super::{
        NaiveDelayLine, NaiveDelayLineParams, TristateInvDelayLine, TristateInvDelayLineParams,
    };

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 1_000,
        pwidth: 1_800,
    };

    const TGATE_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 500,
        pwidth: 900,
    };

    const TRISTATE_SIZING: TristateBufParams = TristateBufParams {
        inv1: INV_SIZING,
        inv2: INV_SIZING,
    };

    const NAIVE_DELAY_LINE_TGATE_PARAMS: NaiveDelayLineParams = NaiveDelayLineParams {
        stages: 100,
        inv1: INV_SIZING,
        inv2: INV_SIZING,
        pass: super::PassGateKind::TransmissionGate(TGATE_SIZING),
    };

    const NAIVE_DELAY_LINE_TRISTATE_PARAMS: NaiveDelayLineParams = NaiveDelayLineParams {
        stages: 100,
        inv1: INV_SIZING,
        inv2: INV_SIZING,
        pass: super::PassGateKind::TristateBuf(TRISTATE_SIZING),
    };

    const TRISTATE_INV_DELAY_LINE_PARAMS: TristateInvDelayLineParams = TristateInvDelayLineParams {
        stages: 128,
        inv: INV_SIZING,
        tristate_inv: INV_SIZING,
    };

    const NAIVE_DELAY_LINE_TGATE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::Naive(NAIVE_DELAY_LINE_TGATE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: Some(10e-8),
    };

    const NAIVE_DELAY_LINE_TRISTATE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::Naive(NAIVE_DELAY_LINE_TRISTATE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: Some(10e-8),
    };

    const TRISTATE_INV_DELAY_LINE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::TristateInv(TRISTATE_INV_DELAY_LINE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: Some(10e-8),
    };

    #[test]
    fn test_naive_delay_line_tgate() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_naive_delay_line_tgate");
        ctx.write_schematic_to_file::<NaiveDelayLine>(
            &NAIVE_DELAY_LINE_TGATE_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
    }

    #[test]
    #[ignore = "slow"]
    fn test_naive_delay_line_tgate_sim() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_naive_delay_line_tgate_sim");
        ctx.write_simulation::<DelayLineTb>(&NAIVE_DELAY_LINE_TGATE_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }

    #[test]
    fn test_naive_delay_line_tristate() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_naive_delay_line_tristate");
        ctx.write_schematic_to_file::<NaiveDelayLine>(
            &NAIVE_DELAY_LINE_TRISTATE_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
    }

    #[test]
    #[ignore = "slow"]
    fn test_naive_delay_line_tristate_sim() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_naive_delay_line_tristate_sim");
        ctx.write_simulation::<DelayLineTb>(&NAIVE_DELAY_LINE_TRISTATE_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }

    #[test]
    fn test_tristate_inv_delay_line() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tristate_inv_delay_line");
        ctx.write_schematic_to_file::<TristateInvDelayLine>(
            &TRISTATE_INV_DELAY_LINE_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<TristateInvDelayLine>(&TRISTATE_INV_DELAY_LINE_PARAMS, &gds_path)
            .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let cell = ctx
                .instantiate_layout::<TristateInvDelayLine>(&TRISTATE_INV_DELAY_LINE_PARAMS)
                .unwrap();
            let name = cell.cell().name();

            let lib_path = crate::paths::out_lib(&work_dir, name);
            save_delay_line_lib(
                lib_path,
                &crate::verilog::DelayLineParams {
                    module_name: name.to_string(),
                    control_width: TRISTATE_INV_DELAY_LINE_PARAMS.stages,
                },
            )
            .expect("failed to write lib file from template");

            let verilog_path = out_verilog(&work_dir, name);
            save_delay_line_verilog(
                &verilog_path,
                &crate::verilog::DelayLineParams {
                    module_name: name.to_string(),
                    control_width: TRISTATE_INV_DELAY_LINE_PARAMS.stages,
                },
            )
            .expect("failed to write behavioral model");

            crate::abs::run_abstract(
                &work_dir,
                name,
                crate::paths::out_lef(&work_dir, name),
                &gds_path,
                &verilog_path,
            )
            .expect("failed to generate abstract");

            let pex_dir = work_dir.join("pex");
            let pex_source_path = out_spice(&pex_dir, "schematic");
            let pex_out_path = out_spice(&pex_dir, "schematic.pex");

            ctx.write_schematic_to_file_for_purpose::<TristateInvDelayLine>(
                &TRISTATE_INV_DELAY_LINE_PARAMS,
                &pex_source_path,
                NetlistPurpose::Pex,
            )
            .expect("failed to write schematic for PEX");

            ctx.run_pex(PexInput {
                work_dir: pex_dir,
                layout_path: gds_path,
                layout_cell_name: name.clone(),
                layout_format: substrate::layout::LayoutFormat::Gds,
                source_paths: vec![pex_source_path],
                source_cell_name: name.clone(),
                pex_netlist_path: pex_out_path,
                opts: Default::default(),
            })
            .expect("failed to run PEX");

            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<TristateInvDelayLine>(&TRISTATE_INV_DELAY_LINE_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<TristateInvDelayLine>(&TRISTATE_INV_DELAY_LINE_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    #[ignore = "slow"]
    fn test_tb_tristate_inv_delay_line() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tb_tristate_inv_delay_line");
        ctx.write_simulation::<DelayLineTb>(&TRISTATE_INV_DELAY_LINE_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }
}
