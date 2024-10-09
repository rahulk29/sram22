use std::collections::{HashMap, HashSet};

use arcstr::ArcStr;
use grid::Grid;
use serde::Serialize;
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::transform::Translate;
use subgeom::{Dir, Rect, Side, Sign, Span};
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{CellPort, Port, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::{OptionTile, Pad, Padding, RectBbox, Tile};
use substrate::layout::routing::manual::jog::{OffsetJog, SJog};
use substrate::layout::DrawRef;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;

use crate::blocks::buf::layout::DiffBufCent;
use crate::blocks::buf::DiffBuf;
use crate::blocks::columns::{Column, ColumnDesignScript};
use crate::blocks::decoder::{
    DecoderPhysicalDesignParams, DecoderStage, DecoderStageParams, DecoderStyle, RoutingStyle,
};
use crate::blocks::gate::{AndParams, GateParams, PrimitiveGateParams};
use crate::blocks::macros::{SenseAmp, SenseAmpCent};
use crate::blocks::precharge::layout::{PrechargeCent, PrechargeEnd, PrechargeEndParams};
use crate::blocks::precharge::Precharge;
use crate::blocks::tgatemux::{TGateMuxCent, TGateMuxEnd, TGateMuxGroup};
use crate::blocks::wrdriver::layout::WriteDriverCent;
use crate::blocks::wrdriver::WriteDriver;

use super::{ColParams, ColPeripherals, WmaskPeripherals};

static BOTTOM_PADDING: Padding = Padding::new(0, 0, 160, 0);

impl ColPeripherals {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let pc_design = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let mut pc = ctx.instantiate::<Precharge>(&self.params.pc)?;
        let mut pc_end = ctx.instantiate::<PrechargeEnd>(&PrechargeEndParams {
            via_top: false,
            inner: self.params.pc,
        })?;

        let col = ctx.instantiate::<Column>(&ColParams {
            include_wmask: false,
            ..self.params.clone()
        })?;
        let bbox = Rect::from_spans(
            Span::new(0, 1_200 * self.params.mux.mux_ratio as i64),
            col.brect().vspan(),
        );
        let col = RectBbox::new(col, bbox);

        let col_wmask = ctx.instantiate::<Column>(&ColParams {
            include_wmask: true,
            ..self.params.clone()
        })?;
        let col_wmask = RectBbox::new(col_wmask, bbox);

        let cent = ctx.instantiate::<ColumnCent>(&ColCentParams {
            col: self.params.clone(),
            end: false,
            cut_wmask: false,
        })?;
        let cent_cut = ctx.instantiate::<ColumnCent>(&ColCentParams {
            col: self.params.clone(),
            end: false,
            cut_wmask: true,
        })?;
        let end = ctx.instantiate::<ColumnCent>(&ColCentParams {
            col: self.params.clone(),
            end: true,
            cut_wmask: false,
        })?;

        let mut row = vec![end.clone().into()];
        let groups = self.params.cols / self.params.mux.mux_ratio;
        let mask_groups = groups / self.params.wmask_granularity;
        let mut col_indices = HashMap::new();
        for i in 0..mask_groups {
            for j in 0..self.params.wmask_granularity {
                col_indices.insert(row.len(), self.params.wmask_granularity * i + j);
                if j == 0 {
                    row.push(col_wmask.clone().into());
                } else {
                    row.push(col.clone().into());
                }
                if !(i == mask_groups - 1 && j == self.params.wmask_granularity - 1) {
                    if j == self.params.wmask_granularity - 1 {
                        row.push(cent_cut.clone().into());
                    } else {
                        row.push(cent.clone().into());
                    }
                }
            }
        }
        row.push(end.with_orientation(Named::ReflectHoriz).into());

        let mut grid = Grid::new(0, 0);
        grid.push_row(row);

        let mut grid_tiler = GridTiler::new(grid);
        grid_tiler.expose_ports(
            |port: CellPort, (_, j)| {
                let port_name = port.name().as_ref();

                match port_name {
                    "bl" | "br" => Some(port.map_index(|index| {
                        col_indices.get(&j).unwrap() * self.params.mux.mux_ratio + index
                    })),
                    "dout" | "din" => Some(port.with_index(*col_indices.get(&j).unwrap())),
                    "we" | "we_b" => Some(port.with_index(*col_indices.get(&j).unwrap())),
                    "pc_b" | "vdd" | "vss" | "sel" | "sel_b" | "sense_en" | "clk" | "reset_b" => {
                        Some(port)
                    }
                    _ => None,
                }
            },
            PortConflictStrategy::Merge,
        )?;

        let group = grid_tiler.draw_ref()?;

        let bbox = group.bbox();
        ctx.draw(group)?;

        let mut wmask_peripherals = ctx.instantiate::<WmaskPeripherals>(&self.params)?;
        wmask_peripherals.align_beneath(bbox, 0);
        wmask_peripherals.align(AlignMode::Left, bbox, pc_design.tap_width / 2);
        ctx.draw_ref(&wmask_peripherals)?;

        // Connect we and we_b to AND gate.
        for i in 0..groups {
            // we
            let wmask_out = wmask_peripherals
                .port(PortId::new("y", i / self.params.wmask_granularity))?
                .largest_rect(m0)?;
            let we_in = grid_tiler
                .port_map()
                .port(PortId::new("we", i))?
                .largest_rect(m1)?;
            let jog = OffsetJog::builder()
                .dir(subgeom::Dir::Vert)
                .sign(subgeom::Sign::Pos)
                .src(wmask_out)
                .dst(we_in.center().x)
                .layer(m0)
                .space(170)
                .build()
                .unwrap();
            let intersect = Rect::from_spans(we_in.hspan(), jog.r2().vspan()).bbox();
            let m1_rect = we_in.bbox().union(intersect).into_rect();
            let m0_rect = jog.r2().bbox().union(intersect).into_rect();
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(m0_rect, m1_rect)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
            ctx.draw_rect(m0, m0_rect);
            ctx.draw_rect(m1, m1_rect);

            ctx.draw(jog)?;

            // we_b
            let wmask_out = wmask_peripherals
                .port(PortId::new("y_b", i / self.params.wmask_granularity))?
                .largest_rect(m0)?;
            let we_in = grid_tiler
                .port_map()
                .port(PortId::new("we_b", i))?
                .largest_rect(m1)?;
            let jog = OffsetJog::builder()
                .dir(subgeom::Dir::Vert)
                .sign(subgeom::Sign::Pos)
                .src(wmask_out)
                .dst(we_in.center().x)
                .layer(m0)
                .space(170)
                .build()
                .unwrap();
            let intersect = Rect::from_spans(we_in.hspan(), jog.r2().vspan()).bbox();
            let m2_rect = we_in
                .with_vspan(Span::with_start_and_length(we_in.bottom(), 300))
                .bbox()
                .union(intersect)
                .into_rect();
            let m0_rect = jog.r2().bbox().union(intersect).into_rect();
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(intersect, intersect)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(via.layer_bbox(m1).into_rect(), m2_rect)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(we_in, m2_rect)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
            ctx.draw_rect(m0, m0_rect);
            ctx.draw_rect(m2, m2_rect);

            ctx.draw(jog)?;
        }

        // Jog dout and din to bottom.
        for i in 0..groups {
            for port in ["dout", "din"] {
                let port_id = PortId::new(port, i);
                let port_rect = grid_tiler
                    .port_map()
                    .port(port_id.clone())?
                    .largest_rect(m1)?;
                let out_rect = Rect::from_spans(
                    Span::from_center_span_gridded(
                        port_rect.center().x,
                        140,
                        ctx.pdk().layout_grid(),
                    ),
                    Span::new(
                        ctx.brect().bottom(),
                        wmask_peripherals.port("we")?.largest_rect(m1)?.center().y - 2000,
                    ),
                );
                let m2_rect =
                    port_rect.with_vspan(Span::new(port_rect.bottom() + 300, out_rect.top() - 300));

                let viap = ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(port_rect, m2_rect)
                    .expand(ViaExpansion::LongerDirection)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw(via)?;
                let viap = ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(out_rect, m2_rect)
                    .expand(ViaExpansion::LongerDirection)
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw(via)?;

                ctx.draw_rect(m1, out_rect);
                ctx.draw_rect(m2, m2_rect);
                ctx.add_port(CellPort::with_shape(port_id, m1, out_rect))?;
            }
        }

        // Route wmask to bottom on m1.
        for i in 0..mask_groups {
            let dff_in = wmask_peripherals
                .port(PortId::new("d", i))?
                .largest_rect(m0)?;
            let wmask_track = Span::with_stop_and_length(
                grid_tiler
                    .port_map()
                    .port(PortId::new("din", i * self.params.wmask_granularity))?
                    .largest_rect(m1)?
                    .left()
                    - 140,
                140,
            );

            let rect1 =
                Rect::from_spans(wmask_track, dff_in.vspan().add_point(ctx.brect().bottom()));
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(dff_in, rect1)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_rect(m1, rect1);
            ctx.draw(via)?;
            ctx.add_port(CellPort::with_shape(PortId::new("wmask", i), m1, rect1))?;
        }

        assert!(!bbox.is_empty());
        pc.align_to_the_left_of(bbox, 0);
        pc.align_top(bbox);
        pc_end.align_to_the_left_of(&pc, 0);
        pc_end.align_top(bbox);

        ctx.draw_ref(&pc)?;
        ctx.draw_ref(&pc_end)?;

        pc.orientation_mut().reflect_horiz();
        pc_end.orientation_mut().reflect_horiz();

        pc.align_to_the_right_of(bbox, 0);
        pc.align_top(bbox);
        pc_end.align_to_the_right_of(&pc, 0);
        pc_end.align_top(bbox);

        ctx.draw_ref(&pc)?;
        ctx.draw_ref(&pc_end)?;

        for port in ["vdd", "vss", "pc_b", "sense_en", "clk", "reset_b"] {
            let spans = grid_tiler
                .port_map()
                .port(port)
                .unwrap()
                .shapes(m2)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| rect.vspan())
                .collect::<HashSet<_>>();
            for span in spans {
                let rect = Rect::from_spans(ctx.brect().hspan(), span);
                ctx.draw_rect(m2, rect);
                ctx.merge_port(CellPort::with_shape(port, m2, rect));
            }
        }
        for port in ["clk", "reset_b", "we"] {
            ctx.merge_port(wmask_peripherals.port(port)?.into_cell_port());
        }
        for port in ["vdd", "vss"] {
            for layer in [m1, m2] {
                for rect in wmask_peripherals
                    .port(port)?
                    .shapes(layer)
                    .filter_map(|shape| shape.as_rect())
                    .filter(|rect| rect.height() < 5000)
                {
                    let full_span_port = rect.with_hspan(ctx.brect().hspan());
                    ctx.draw_rect(layer, full_span_port);
                    ctx.merge_port(CellPort::with_shape(port, layer, full_span_port));
                }
            }
        }
        for i in 0..self.params.mux_ratio() {
            for port in ["sel", "sel_b"] {
                ctx.merge_port(grid_tiler.port_map().port(PortId::new(port, i))?.clone());
            }
        }
        for i in 0..self.params.cols {
            for port in ["bl", "br"] {
                ctx.merge_port(grid_tiler.port_map().port(PortId::new(port, i))?.clone());
            }
        }

        Ok(())
    }
}

impl WmaskPeripherals {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let outline = layers.get(Selector::Name("outline"))?;

        let pc_design = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;
        let wmask_unit_width = self.params.wmask_granularity as i64
            * (pc_design.width * self.params.mux_ratio() as i64 + pc_design.tap_width);

        let nand_stage = ctx.instantiate::<DecoderStage>(&DecoderStageParams {
            pd: DecoderPhysicalDesignParams {
                style: DecoderStyle::Minimum,
                dir: Dir::Horiz,
            },
            routing_style: RoutingStyle::Decoder,
            max_width: Some(wmask_unit_width),
            gate: GateParams::And2(AndParams {
                inv: PrimitiveGateParams {
                    nwidth: 10000,
                    pwidth: 10000,
                    length: 150,
                },
                nand: PrimitiveGateParams {
                    nwidth: 2000,
                    pwidth: 2000,
                    length: 150,
                },
            }),
            invs: vec![],
            num: 1,
            child_sizes: vec![1, 1],
        })?;
        let wmask_dff = ctx.instantiate::<DffCol>(&NoParams)?;

        let mut grid = Grid::new(0, 0);
        let mut row = vec![];
        for _ in 0..self.params.wmask_bits() {
            row.push(
                RectBbox::new(
                    nand_stage.clone(),
                    nand_stage.brect().with_hspan(Span::with_start_and_length(
                        nand_stage.brect().left(),
                        wmask_unit_width,
                    )),
                )
                .into(),
            );
        }
        grid.push_row(row);
        let mut row = vec![];
        let offset = (self.params.mux_ratio() - 4) / 2;
        for _ in 0..self.params.wmask_bits() {
            let wmask_dff_brect = wmask_dff.layer_bbox(outline).into_rect();
            row.push(
                RectBbox::new(
                    wmask_dff.clone(),
                    wmask_dff_brect.with_hspan(Span::with_start_and_length(
                        wmask_dff_brect.left() - offset as i64 * pc_design.width,
                        wmask_unit_width,
                    )),
                )
                .into(),
            );
        }
        grid.push_row(row);
        let mut wmask_grid_tiler = GridTiler::new(grid);
        wmask_grid_tiler.expose_ports(
            |port: CellPort, (_, j)| Some(port.with_index(j)),
            PortConflictStrategy::Merge,
        )?;
        ctx.draw_ref(&wmask_grid_tiler)?;

        for (original_port, new_port, layer) in [
            ("predecode_0_0", "we", m1),
            ("vdd", "vdd", m1),
            ("vdd", "vdd", m2),
            ("vss", "vss", m1),
            ("vss", "vss", m2),
            ("clk", "clk", m2),
            ("reset_b", "reset_b", m2),
        ] {
            let spans = wmask_grid_tiler
                .port_map()
                .port(PortId::new(original_port, 0))
                .unwrap()
                .shapes(layer)
                .filter_map(|shape| shape.as_rect())
                .map(|rect| rect.vspan());
            for span in spans {
                if span.length() < 5000 {
                    let rect = Rect::from_spans(ctx.brect().hspan(), span);
                    ctx.draw_rect(layer, rect);
                    ctx.merge_port(CellPort::with_shape(new_port, layer, rect));
                }
            }
        }

        for i in 0..self.params.wmask_bits() {
            ctx.add_port(
                wmask_grid_tiler
                    .port_map()
                    .port(PortId::new("y", i))?
                    .clone(),
            )?;
            ctx.add_port(
                wmask_grid_tiler
                    .port_map()
                    .port(PortId::new("y_b", i))?
                    .clone(),
            )?;
        }

        for i in 0..self.params.wmask_bits() {
            let dff_out = wmask_grid_tiler
                .port_map()
                .port(PortId::new("q", i))?
                .largest_rect(m0)?;
            let wmask_in = wmask_grid_tiler
                .port_map()
                .port(PortId::new("predecode_1_0", i))?
                .largest_rect(m1)?;

            let m1_track =
                Span::from_center_span_gridded(dff_out.center().x, 280, ctx.pdk().layout_grid());
            let m1_rect = Rect::from_spans(m1_track, dff_out.vspan().union(wmask_in.vspan()));
            ctx.draw_rect(m1, m1_rect);

            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(dff_out, m1_rect)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            ctx.add_port(
                wmask_grid_tiler
                    .port_map()
                    .port(PortId::new("d", i))?
                    .clone(),
            )?;
        }

        Ok(())
    }
}

impl Column {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let pc_design = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;
        let mut dff = ctx.instantiate::<DffCol>(&NoParams)?;
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let mux_ratio = self.params.mux.mux_ratio;
        let mut pc = ctx.instantiate::<Precharge>(&self.params.pc)?;
        let mut grid = Grid::new(0, 0);
        let mut row = Vec::new();
        for _ in 0..mux_ratio / 2 {
            let pc1 = pc.with_orientation(Named::ReflectHoriz);
            let pc2 = pc.clone();
            row.push(pc1.into());
            row.push(pc2.into());
        }
        grid.push_row(row);

        let mut mux = ctx.instantiate::<TGateMuxGroup>(&self.params.mux)?;
        let bbox = Rect::from_spans(
            pc.brect().hspan(),
            mux.layer_bbox(outline).into_rect().vspan(),
        );

        let mut row = Vec::new();
        row.push(OptionTile::new(Tile::from(Pad::new(
            RectBbox::new(mux.clone(), bbox),
            BOTTOM_PADDING,
        ))));
        for _ in 0..mux_ratio - 1 {
            row.push(None.into());
        }
        grid.push_row(row);

        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.orientation_mut().reflect_vert();
        let bbox = Rect::from_spans(pc.brect().hspan(), sa.brect().vspan());

        let mut row = Vec::new();
        let offset = (mux_ratio - 4) / 2;
        for _ in 0..offset {
            row.push(None.into());
        }
        row.push(OptionTile::new(Tile::from(RectBbox::new(sa.clone(), bbox))));
        for _ in offset + 1..mux_ratio {
            row.push(None.into());
        }
        grid.push_row(row);

        let mut wrdrv = ctx.instantiate::<WriteDriver>(&self.params.wrdriver)?;
        let bbox = Rect::from_spans(
            Span::with_start_and_length(wrdrv.brect().left(), pc.brect().width()),
            wrdrv.brect().vspan(),
        );

        let mut row = Vec::new();
        for _ in 0..offset {
            row.push(None.into());
        }
        row.push(OptionTile::new(Tile::from(RectBbox::new(
            wrdrv.clone(),
            bbox,
        ))));
        for _ in offset + 1..mux_ratio {
            row.push(None.into());
        }
        grid.push_row(row);

        let mut buf = ctx.instantiate::<DiffBuf>(&self.params.buf)?;
        let bbox = Rect::from_spans(
            Span::with_start_and_length(buf.brect().left(), pc.brect().width()),
            buf.brect().vspan(),
        );

        let mut row = Vec::new();
        for _ in 0..offset {
            row.push(None.into());
        }
        row.push(OptionTile::new(Tile::from(RectBbox::new(
            buf.clone(),
            bbox,
        ))));
        for _ in offset + 1..mux_ratio {
            row.push(None.into());
        }
        grid.push_row(row);

        // Data dff
        let bbox = Rect::from_spans(
            Span::with_start_and_length(
                dff.layer_bbox(outline).into_rect().left() + pc_design.tap_width / 2,
                pc.brect().width(),
            ),
            dff.layer_bbox(outline).into_rect().vspan(),
        );

        let mut row = Vec::new();
        for _ in 0..offset {
            row.push(None.into());
        }
        row.push(OptionTile::new(Tile::from(RectBbox::new(
            dff.clone(),
            bbox,
        ))));
        for _ in offset + 1..mux_ratio {
            row.push(None.into());
        }
        grid.push_row(row);

        let mut tiler = GridTiler::new(grid);
        pc.translate(tiler.translation(0, 0));
        mux.translate(tiler.translation(1, 0));
        sa.translate(tiler.translation(2, offset));
        wrdrv.translate(tiler.translation(3, offset));
        buf.translate(tiler.translation(4, offset));
        dff.translate(tiler.translation(5, offset));
        tiler.expose_ports(
            |port: CellPort, (i, j)| match port.name().as_str() {
                "br_in" => Some(port.named("br").with_index(j)),
                "bl_in" => Some(port.named("bl").with_index(j)),
                "reset_b" | "sel" | "sel_b" | "vdd" | "vss" => Some(port),
                "en_b" => {
                    if i == 0 {
                        Some(port.named("pc_b"))
                    } else {
                        None
                    }
                }
                "clk" => {
                    if i == 2 {
                        Some(port.named("sense_en"))
                    } else {
                        Some(port)
                    }
                }
                _ => None,
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();
        ctx.draw(tiler)?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;

        // Route sense amp inputs to bitlines.
        for (tgate_port, sa_port) in [("bl_out", "inp"), ("br_out", "inn")] {
            let sa_rect = sa.port(sa_port)?.largest_rect(m1)?;
            let tgate_rect = mux.port(tgate_port)?.largest_rect(m2)?;
            let m1_rect = sa_rect.with_vspan(sa_rect.vspan().union(tgate_rect.vspan()));

            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(m1_rect, tgate_rect)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;
            ctx.draw_rect(m1, m1_rect);
        }

        // Route positive diffbuf output to bottom.
        let buf_out = buf.port("dout1")?.largest_rect(m0)?;
        let dff_vss = dff.port("vss")?.largest_rect(m1)?;
        let dout_track = Span::with_stop_and_length(dff_vss.left() - 140, 280);
        let center_track =
            Span::from_center_span_gridded(buf.brect().center().x, 280, ctx.pdk().layout_grid());

        let jog_y = dff.port("q")?.largest_rect(m0)?.bottom() - 600;
        let jog = OffsetJog::builder()
            .dir(subgeom::Dir::Vert)
            .sign(subgeom::Sign::Neg)
            .src(buf_out)
            .dst(center_track.start())
            .layer(m0)
            .space(300)
            .build()
            .unwrap();
        let rect1 = Rect::from_spans(center_track, jog.r2().vspan().add_point(jog_y));
        let rect2 = Rect::from_spans(dout_track, Span::new(ctx.brect().bottom(), jog_y));
        let rect3 = Rect::from_spans(
            dout_track.union(center_track),
            Span::from_center_span_gridded(jog_y, 280, ctx.pdk().layout_grid()),
        );
        let viap = ViaParams::builder()
            .layers(m0, m1)
            .geometry(jog.r2(), rect1)
            .build();
        let via = ctx.instantiate::<Via>(&viap)?;
        ctx.draw(jog)?;
        ctx.draw_rect(m1, rect1);
        ctx.draw_rect(m1, rect2);
        ctx.draw_rect(m1, rect3);
        ctx.draw(via)?;
        ctx.add_port(CellPort::with_shape("dout", m1, rect2))?;

        // Route dff input to bottom.
        let dff_in = dff.port("d")?.largest_rect(m0)?;
        let din_track = Span::with_stop_and_length(dout_track.start() - 140, 280);

        let rect1 = Rect::from_spans(din_track, dff_in.vspan().add_point(ctx.brect().bottom()));
        let viap = ViaParams::builder()
            .layers(m0, m1)
            .geometry(dff_in, rect1)
            .build();
        let via = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_rect(m1, rect1);
        ctx.draw(via)?;
        ctx.add_port(CellPort::with_shape("din", m1, rect1))?;

        // Route din and din_b to dff.
        let dout2 = buf.port("dout2")?.largest_rect(m0)?;
        for (in_port, out_port, center) in [
            ("data", "q", buf.port("dout1")?.largest_rect(m0)?.center().x),
            ("data_b", "q_n", dout2.center().x),
        ] {
            let port_rect = wrdrv.port(in_port)?.largest_rect(m0)?;
            let out_port_rect = if out_port == "q" {
                dff.port(out_port)?.largest_rect(m0)?
            } else {
                dff.port(out_port)?.first_rect(m0, Side::Left)?
            };
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(port_rect, port_rect)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;

            let m1_track = Span::from_center_span_gridded(center, 280, ctx.pdk().layout_grid());
            let m1_rect = Rect::from_spans(m1_track, dout2.vspan().union(out_port_rect.vspan()));
            let jog = SJog::builder()
                .src(m1_rect)
                .dst(via.layer_bbox(m1).into_rect())
                .dir(Dir::Vert)
                .layer(m1)
                .grid(ctx.pdk().layout_grid())
                .build()
                .unwrap();
            ctx.draw(jog)?;
            ctx.draw_rect(m1, m1_rect);
            let viap = ViaParams::builder()
                .layers(m0, m1)
                .geometry(out_port_rect, m1_rect)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
        }

        // Route we and we_b to bottom
        for (side, port, in_port) in [(Sign::Neg, "en", "we"), (Sign::Pos, "en_b", "we_b")] {
            let port_rect = wrdrv.port(port)?.largest_rect(m2)?;
            let dff_m1_brect = dff.layer_bbox(m1).into_rect();
            let dout_track = Span::with_point_and_length(
                !side,
                dff_m1_brect.hspan().point(side) + side.as_int() * 140,
                280,
            );
            let m2_rect = Rect::from_spans(dout_track.union(port_rect.hspan()), port_rect.vspan());
            let m1_rect = Rect::from_spans(
                dout_track,
                port_rect.vspan().add_point(ctx.brect().bottom()),
            );
            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(m1_rect, m2_rect)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_rect(m1, m1_rect);
            ctx.add_port(CellPort::builder().id(in_port).add(m1, m1_rect).build())?;
            ctx.draw_rect(m2, m2_rect);
            ctx.draw(via)?;
        }

        // Expand nwells
        for inst in [&sa, &wrdrv, &buf] {
            for shape in inst.shapes_on(nwell) {
                ctx.draw_rect(nwell, shape.brect().with_hspan(ctx.brect().hspan()));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ColCentParams {
    pub col: ColParams,
    pub end: bool,
    pub cut_wmask: bool,
}

pub struct ColumnCent {
    params: ColCentParams,
}

impl Component for ColumnCent {
    type Params = ColCentParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column_cent")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        // Always use a precharge center tile; the real precharge end
        // is used for the replica and dummy column.
        let mut pc = ctx.instantiate::<PrechargeCent>(&self.params.col.pc)?;
        let mut mux = if self.params.end {
            ctx.instantiate::<TGateMuxEnd>(&self.params.col.mux)?
        } else {
            ctx.instantiate::<TGateMuxCent>(&self.params.col.mux)?
        };
        let mut sa = ctx.instantiate::<SenseAmpCent>(&NoParams)?;
        sa.set_orientation(Named::ReflectVert);
        let mut wrdrv = ctx.instantiate::<WriteDriverCent>(&self.params.col.wrdriver)?;
        let mut buf = ctx.instantiate::<DiffBufCent>(&self.params.col.buf)?;
        let mut dff = ctx.instantiate::<DffColCent>(&NoParams)?;
        let mut grid = Grid::new(0, 0);
        grid.push_row(into_vec![pc.clone()]);
        grid.push_row(into_vec![Pad::new(
            RectBbox::new(mux.clone(), mux.layer_bbox(outline).into_rect()),
            BOTTOM_PADDING
        )]);
        grid.push_row(into_vec![sa.clone()]);
        grid.push_row(into_vec![wrdrv.clone()]);
        grid.push_row(into_vec![buf.clone()]);
        grid.push_row(into_vec![dff.clone()]);

        let mut tiler = GridTiler::new(grid);
        pc.translate(tiler.translation(0, 0));
        mux.translate(tiler.translation(1, 0));
        sa.translate(tiler.translation(2, 0));
        wrdrv.translate(tiler.translation(3, 0));
        buf.translate(tiler.translation(4, 0));
        dff.translate(tiler.translation(5, 0));
        tiler.expose_ports(
            |port: CellPort, (i, _)| match port.name().as_str() {
                "sel" | "sel_b" | "vdd" | "vss" => Some(port),
                "en_b" => {
                    if i == 0 {
                        Some(port.named("pc_b"))
                    } else {
                        Some(port.named("we_b"))
                    }
                }
                "en" => Some(port.named("we")),
                "clk" => {
                    if i == 2 {
                        Some(port.named("sense_en"))
                    } else {
                        Some(port)
                    }
                }
                _ => None,
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned())?;
        ctx.draw(tiler)?;

        Ok(())
    }
}

pub struct TappedDff;

impl Component for TappedDff {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_dff")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let dff = lib.try_cell_named("sky130_fd_sc_hs__dfrbp_2")?;
        let dff = ctx
            .instantiate::<StdCell>(&dff.id())?
            .with_orientation(Named::R90);
        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let outline = layers.get(Selector::Name("outline"))?;
        let tap = layers.get(Selector::Name("tap"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let bbox = dff.layer_bbox(outline).into_rect();

        let hspan = Span::from_center_span_gridded(
            bbox.center().x,
            4 * pc.width + pc.tap_width,
            ctx.pdk().layout_grid(),
        );
        let nwell_bbox = dff.layer_bbox(nwell).into_rect();
        ctx.draw_rect(
            nwell,
            Rect::from_spans(
                Span::new(hspan.start(), nwell_bbox.left()),
                nwell_bbox.vspan(),
            ),
        );

        for (side, vdd) in [(Sign::Neg, true), (Sign::Pos, false)] {
            let r = Rect::from_spans(
                Span::new(bbox.hspan().point(side), hspan.point(side)).shrink_all(200),
                bbox.vspan(),
            )
            .shrink(200);
            let viap = ViaParams::builder().layers(tap, m0).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;

            ctx.draw_ref(&via)?;
            let sdm_rect = via.layer_bbox(tap).into_rect().expand(130);
            ctx.draw_rect(if vdd { nsdm } else { psdm }, sdm_rect);
            let m0_bbox = via.layer_bbox(m0).into_rect();
            ctx.draw_rect(
                m0,
                m0_bbox.with_hspan(m0_bbox.hspan().add_point(bbox.hspan().point(side))),
            );

            let port = if vdd { "vpwr" } else { "vgnd" };
            let m1_rect = dff.port(port)?.largest_rect(m1)?;

            let port = if vdd { "vdd" } else { "vss" };
            ctx.merge_port(CellPort::with_shape(port, m1, m1_rect));
        }

        // Route clock/reset to metal 2 tracks.
        let clk_rect = dff.port("clk")?.largest_rect(m0)?;
        let clk_rect = clk_rect.with_hspan(clk_rect.hspan().shrink(Sign::Neg, 220));
        let viap = ViaParams::builder()
            .layers(m0, m1)
            .geometry(clk_rect, clk_rect)
            .expand(ViaExpansion::LongerDirection)
            .build();
        let via = ctx.instantiate::<Via>(&viap)?;
        ctx.draw_ref(&via)?;
        for (port, geometry) in [
            ("clk", via.layer_bbox(m1).into_rect()),
            ("reset_b", dff.port("reset_b")?.largest_rect(m1)?),
        ] {
            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(geometry, geometry)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
            let stripe = Rect::from_spans(hspan, via.layer_bbox(m2).into_rect().vspan());
            ctx.draw_rect(m2, stripe);
            ctx.add_port(CellPort::with_shape(port, m2, stripe))?;
        }

        for port in ["q", "q_n", "clk", "reset_b", "d"] {
            ctx.merge_port(dff.port(port)?.into_cell_port());
        }
        ctx.draw(dff)?;
        Ok(())
    }
}

pub struct DffCol;

impl Component for DffCol {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("dff_col")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dff = ctx.instantiate::<TappedDff>(&NoParams)?;
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let bbox = dff.layer_bbox(outline).into_rect();

        let hspan = Span::from_center_span_gridded(
            bbox.center().x,
            4 * pc.width + pc.tap_width,
            ctx.pdk().layout_grid(),
        );
        for (side, vdd) in [(Sign::Neg, true), (Sign::Pos, false)] {
            let power_stripe = Rect::from_spans(
                hspan,
                Span::from_center_span_gridded(
                    bbox.center().y + side.as_int() * 1870,
                    1800,
                    ctx.pdk().layout_grid(),
                ),
            );

            let port = if vdd { "vdd" } else { "vss" };
            let m1_rect = dff.port(port)?.largest_rect(m1)?;
            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(m1_rect, power_stripe)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            ctx.draw_rect(m2, power_stripe);

            ctx.merge_port(CellPort::with_shape(port, m2, power_stripe));
            ctx.merge_port(CellPort::with_shape(port, m1, m1_rect));
        }
        ctx.draw_rect(
            outline,
            dff.brect().with_hspan(hspan).expand_dir(Dir::Vert, 1270),
        );

        for port in ["q", "q_n", "clk", "reset_b", "d"] {
            ctx.merge_port(dff.port(port)?.into_cell_port());
        }
        ctx.draw(dff)?;
        Ok(())
    }
}

pub struct DffArray {
    n: usize,
}

impl Component for DffArray {
    type Params = usize;
    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self { n: *params })
    }
    fn name(&self) -> ArcStr {
        arcstr::format!("dff_array_{}", self.n)
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let n = self.n;
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let clk = ctx.port("clk", Direction::Input);
        let rb = ctx.port("rb", Direction::Input);
        let d = ctx.bus_port("d", n, Direction::Input);
        let q = ctx.bus_port("q", n, Direction::Output);
        let qn = ctx.bus_port("qn", n, Direction::Output);

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let dfrtp = lib.try_cell_named("sky130_fd_sc_hs__dfrbp_2")?;

        for i in 0..self.n {
            ctx.instantiate::<StdCell>(&dfrtp.id())?
                .with_connections([
                    ("VPWR", vdd),
                    ("VGND", vss),
                    ("VNB", vss),
                    ("VPB", vdd),
                    ("CLK", clk),
                    ("RESET_B", rb),
                    ("D", d.index(i)),
                    ("Q", q.index(i)),
                    ("Q_N", qn.index(i)),
                ])
                .named(format!("dff_{i}"))
                .add_to(ctx);
        }

        Ok(())
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dff = ctx.instantiate::<TappedDff>(&NoParams)?;
        let mut tiler = ArrayTiler::builder()
            .mode(AlignMode::ToTheRight)
            .push_num(dff, self.n)
            .build();

        tiler.expose_ports(
            |port: CellPort, i| {
                if ["vdd", "vss", "clk", "reset_b"].contains(&port.name().as_ref()) {
                    Some(port)
                } else {
                    let port = port.with_index(i);
                    Some(port)
                }
            },
            substrate::layout::cell::PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();

        ctx.draw(tiler)?;
        Ok(())
    }
}

pub struct DffColCent;

impl Component for DffColCent {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("dff_col")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let dff = ctx.instantiate::<DffCol>(&NoParams)?;

        let pc = ctx.inner().run_script::<ColumnDesignScript>(&NoParams)?;

        let bbox = dff.layer_bbox(outline).into_rect();

        let hspan = Span::new(0, pc.tap_width);

        for port in ["vdd", "vss", "clk", "reset_b"] {
            let r = Rect::from_spans(hspan, dff.port(port)?.largest_rect(m2)?.vspan());
            ctx.draw_rect(m2, r);
            ctx.merge_port(CellPort::with_shape(port, m2, r));
        }
        ctx.draw_rect(outline, bbox.with_hspan(hspan));
        Ok(())
    }
}
