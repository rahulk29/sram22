use std::collections::HashMap;

use grid::Grid;
use serde::{Deserialize, Serialize};
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Named;
use subgeom::transform::Translate;
use subgeom::{Rect, Span};
use substrate::component::{Component, NoParams};
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{CellPort, Instance, Port, PortConflictStrategy};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::{OptionTile, Pad, Padding, RectBbox, Tile};
use substrate::layout::routing::tracks::{Boundary, CenteredTrackParams, FixedTracks};
use substrate::layout::Draw;

use crate::blocks::buf::layout::DiffBufCent;
use crate::blocks::buf::DiffBuf;
use crate::blocks::columns::Column;
use crate::blocks::macros::{DffCol, DffColCent, DffColExtend, SenseAmp, SenseAmpCent};
use crate::blocks::precharge::layout::{PrechargeCent, PrechargeEnd, PrechargeEndParams};
use crate::blocks::precharge::Precharge;
use crate::blocks::rmux::{ReadMux, ReadMuxCent, ReadMuxEnd, ReadMuxParams};
use crate::blocks::wmux::{
    WriteMux, WriteMuxCent, WriteMuxCentParams, WriteMuxEnd, WriteMuxEndParams, WriteMuxParams,
};

use super::{ColParams, ColPeripherals};

static DFF_PADDING: Padding = Padding::new(160, 0, 0, 0);

impl ColPeripherals {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m2 = layers.get(Selector::Metal(2))?;

        let mut pc = ctx.instantiate::<Precharge>(&self.params.pc)?;
        let mut pc_end = ctx.instantiate::<PrechargeEnd>(&PrechargeEndParams {
            via_top: false,
            inner: self.params.pc.clone(),
        })?;

        let col = ctx.instantiate::<Column>(&ColParams {
            include_wmask: false,
            ..self.params.clone()
        })?;
        let bbox = Rect::from_spans(
            Span::new(0, 1_200 * self.params.wmux.mux_ratio as i64),
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
        let groups = self.params.cols / self.params.wmux.mux_ratio;
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
                        col_indices.get(&j).unwrap() * self.params.wmux.mux_ratio + index
                    })),
                    "dout" | "din" => Some(port.with_index(*col_indices.get(&j).unwrap())),
                    "wmask" => {
                        Some(port.with_index(
                            *col_indices.get(&j).unwrap() / self.params.wmask_granularity,
                        ))
                    }
                    "en_b" => Some(port.named("pc_b")),
                    _ => Some(port),
                }
            },
            PortConflictStrategy::Merge,
        )?;

        ctx.add_ports(grid_tiler.ports().cloned()).unwrap();

        for port_name in ["vdd", "vss", "sense_en"] {
            let bboxes = grid_tiler.port_map().port(port_name)?.shapes(m2).fold(
                HashMap::new(),
                |mut acc, shape| {
                    let entry = acc.entry(shape.brect().vspan()).or_insert(Bbox::empty());
                    *entry = entry.union(shape.bbox());
                    acc
                },
            );
            for bbox in bboxes.values() {
                ctx.merge_port(CellPort::with_shape(port_name, m2, bbox.into_rect()));
            }
        }

        let group = grid_tiler.draw()?;

        let bbox = group.bbox();
        ctx.draw(group)?;

        assert!(!bbox.is_empty());
        pc.align_to_the_left_of(bbox, 0);
        pc.align_top(bbox);
        pc_end.align_to_the_left_of(&pc, 0);
        pc_end.align_top(bbox);

        ctx.draw_ref(&pc)?;
        ctx.draw_ref(&pc_end)?;
        ctx.merge_port(pc.port("en_b")?.into_cell_port().named("pc_b"));
        ctx.merge_port(pc_end.port("en_b")?.into_cell_port().named("pc_b"));
        ctx.add_port(pc.port("bl_in")?.into_cell_port().named("dummy_bl_in"))?;
        ctx.add_port(pc.port("br_in")?.into_cell_port().named("dummy_br_in"))?;
        ctx.add_port(pc.port("bl_out")?.into_cell_port().named("dummy_bl"))?;
        ctx.add_port(pc.port("br_out")?.into_cell_port().named("dummy_br"))?;

        pc.orientation_mut().reflect_horiz();
        pc_end.orientation_mut().reflect_horiz();

        pc.align_to_the_right_of(bbox, 0);
        pc.align_top(bbox);
        pc_end.align_to_the_right_of(&pc, 0);
        pc_end.align_top(bbox);

        ctx.draw_ref(&pc)?;
        ctx.draw_ref(&pc_end)?;
        ctx.merge_port(pc.port("en_b")?.into_cell_port().named("pc_b"));
        ctx.merge_port(pc_end.port("en_b")?.into_cell_port().named("pc_b"));

        Ok(())
    }
}

impl Column {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        let mux_ratio = self.params.rmux.mux_ratio;
        let mut pc = ctx.instantiate::<Precharge>(&self.params.pc)?;
        let mut rmux = ctx.instantiate::<ReadMux>(&ReadMuxParams {
            idx: 0,
            ..self.params.rmux.clone()
        })?;
        let mut wmux = ctx.instantiate::<WriteMux>(&WriteMuxParams {
            sizing: self.params.wmux,
            idx: 0,
        })?;
        let mut grid = Grid::new(0, 0);
        let mut row = Vec::new();
        for _ in 0..mux_ratio / 2 {
            let pc1 = pc.with_orientation(Named::ReflectHoriz);
            let pc2 = pc.clone();
            row.push(pc1.into());
            row.push(pc2.into());
        }
        grid.push_row(row);
        let mut row = Vec::new();
        for i in (0..mux_ratio).step_by(2) {
            let rmux1 = ctx.instantiate::<ReadMux>(&ReadMuxParams {
                idx: i,
                ..self.params.rmux.clone()
            })?;
            let mut rmux2 = ctx.instantiate::<ReadMux>(&ReadMuxParams {
                idx: i + 1,
                ..self.params.rmux.clone()
            })?;
            rmux2.orientation_mut().reflect_horiz();
            row.push(rmux1.into());
            row.push(rmux2.into());
        }
        grid.push_row(row);

        let mut row = Vec::new();
        for i in (0..mux_ratio).step_by(2) {
            let wmux1 = ctx.instantiate::<WriteMux>(&WriteMuxParams {
                sizing: self.params.wmux,
                idx: i,
            })?;
            let mut wmux2 = ctx.instantiate::<WriteMux>(&WriteMuxParams {
                sizing: self.params.wmux,
                idx: i + 1,
            })?;
            wmux2.orientation_mut().reflect_horiz();

            row.push(wmux1.into());
            row.push(wmux2.into());
        }
        grid.push_row(row);

        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.orientation_mut().reflect_vert();
        let bbox = Rect::from_spans(pc.brect().hspan(), sa.brect().vspan());

        let mut row = Vec::new();
        row.push(OptionTile::new(Tile::from(RectBbox::new(sa.clone(), bbox))));
        for _ in 0..mux_ratio - 1 {
            row.push(None.into());
        }
        grid.push_row(row);

        let mut buf = ctx.instantiate::<DiffBuf>(&self.params.buf)?;
        buf.set_orientation(Named::R90Cw);
        let bbox = Rect::from_spans(pc.brect().hspan(), buf.brect().vspan());

        let mut row = Vec::new();
        row.push(OptionTile::new(Tile::from(RectBbox::new(
            buf.clone(),
            bbox,
        ))));
        for _ in 0..mux_ratio - 1 {
            row.push(None.into());
        }
        grid.push_row(row);

        // Data dff
        let mut dff = ctx.instantiate::<DffCol>(&NoParams)?;
        let mut wmask_dff = ctx.instantiate::<DffCol>(&NoParams)?;
        let cent = ctx.instantiate::<DffColExtend>(&NoParams)?;
        let bbox = Rect::from_spans(
            Span::with_start_and_length((5_840 - 4_800) / 2, pc.brect().width()),
            dff.brect().vspan(),
        );

        let mut row = Vec::new();
        row.push(OptionTile::new(Tile::from(RectBbox::new(
            dff.clone(),
            bbox,
        ))));
        for _ in 0..mux_ratio - 1 {
            row.push(cent.clone().into());
        }
        grid.push_row(row);
        let mut row = Vec::new();
        if self.params.include_wmask {
            row.push(OptionTile::new(Tile::from(Pad::new(
                RectBbox::new(dff.clone(), bbox),
                DFF_PADDING,
            ))));
        } else {
            row.push(Pad::new(cent.clone(), DFF_PADDING).into());
        }
        for _ in 0..mux_ratio - 1 {
            row.push(Pad::new(cent.clone(), DFF_PADDING).into());
        }
        grid.push_row(row);

        let mut tiler = GridTiler::new(grid);
        pc.translate(tiler.translation(0, 0));
        rmux.translate(tiler.translation(1, 0));
        wmux.translate(tiler.translation(2, 0));
        sa.translate(tiler.translation(3, 0));
        buf.translate(tiler.translation(4, 0));
        dff.translate(tiler.translation(5, 0));
        if self.params.include_wmask {
            wmask_dff.translate(tiler.translation(6, 0));
        }
        tiler.expose_ports(
            |port: CellPort, (i, j)| match i {
                0..=2 => match port.name().as_str() {
                    "bl_in" => Some(port.named("bl").with_index(j)),
                    "br_in" => Some(port.named("br").with_index(j)),
                    "en_b" | "we" | "sel_b" => Some(port),
                    _ => None,
                },
                3 => match port.name().as_str() {
                    "clk" => Some(port.named("sense_en")),
                    _ => None,
                },
                5 | 6 => match port.name().as_str() {
                    "clk" => Some(port.with_index(i - 5)),
                    _ => None,
                },
                _ => None,
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();
        ctx.draw(tiler)?;

        let hspan = Span::new(0, 4 * pc.brect().width());
        let tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 400,
            space: 400,
            num: 6,
            span: hspan,
            lower_boundary: Boundary::HalfSpace,
            upper_boundary: Boundary::HalfSpace,
            grid: 5,
        });

        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;
        let vspan = ctx.brect().vspan();

        let track_vspans = |track: CellTrack| -> substrate::error::Result<Vec<Span>> {
            use CellTrack::*;
            let pad = 40;
            Ok(match track {
                ReadP => vec![
                    Span::new(
                        sa.port("inp")?.largest_rect(m2)?.bottom() - pad,
                        rmux.port("read_bl")?.largest_rect(m2)?.top() + pad,
                    ),
                    Span::new(
                        buf.port("inp")?.largest_rect(m2)?.bottom() - pad,
                        sa.port("outp")?.largest_rect(m2)?.top() + pad,
                    ),
                    Span::new(
                        vspan.start(),
                        buf.port("outp")?.largest_rect(m2)?.top() + pad,
                    ),
                ],
                ReadN => vec![
                    Span::new(
                        sa.port("inn")?.largest_rect(m2)?.bottom() - pad,
                        rmux.port("read_br")?.largest_rect(m2)?.top() + pad,
                    ),
                    Span::new(
                        buf.port("inn")?.largest_rect(m2)?.bottom() - pad,
                        sa.port("outn")?.largest_rect(m2)?.top() + pad,
                    ),
                ],
                DataIn => vec![Span::new(
                    vspan.start(),
                    dff.port("d")?.largest_rect(m2)?.top() + pad,
                )],
                Data => {
                    if self.params.include_wmask {
                        vec![
                            Span::new(
                                dff.port("q")?.largest_rect(m2)?.bottom() - pad,
                                wmux.brect().top(),
                            ),
                            Span::new(
                                vspan.start(),
                                wmask_dff.port("d")?.largest_rect(m2)?.top() + pad,
                            ),
                        ]
                    } else {
                        vec![Span::new(
                            dff.port("q")?.largest_rect(m2)?.bottom() - pad,
                            wmux.brect().top(),
                        )]
                    }
                }
                DataB => vec![Span::new(
                    dff.port("qb")?.largest_rect(m2)?.bottom() - pad,
                    wmux.brect().top(),
                )],
                Wmask => {
                    if self.params.include_wmask {
                        vec![Span::new(
                            wmask_dff.port("q")?.largest_rect(m2)?.bottom() - pad,
                            wmux.brect().top(),
                        )]
                    } else {
                        vec![]
                    }
                }
            })
        };

        for (i, track) in tracks.iter().enumerate() {
            let name = CellTrack::from(i);
            let vspans = track_vspans(name)?;
            for vspan in vspans.iter() {
                let rect = Rect::from_spans(track, *vspan);
                ctx.draw_rect(m3, rect);
            }

            if let Some(vspan) = vspans.last() {
                ctx.add_port(
                    CellPort::builder()
                        .id(match name {
                            CellTrack::ReadP => "dout",
                            CellTrack::DataIn => "din",
                            CellTrack::Data => {
                                if self.params.include_wmask {
                                    "wmask"
                                } else {
                                    continue;
                                }
                            }
                            _ => continue,
                        })
                        .add(m3, Rect::from_spans(track, *vspan))
                        .build(),
                )?;
            }
        }

        for shape in sa.shapes_on(nwell) {
            ctx.draw_rect(nwell, shape.brect().with_hspan(ctx.brect().hspan()));
        }

        let mut draw_vias =
            |inst: &Instance, port: &str, track: CellTrack| -> substrate::error::Result<()> {
                let idx = track.into();
                let port = inst.port(port)?;
                for shape in port.shapes(m2) {
                    let target_vspan = shape.brect().vspan();
                    let viap = ViaParams::builder()
                        .layers(m2, m3)
                        .geometry(
                            Rect::from_spans(hspan, target_vspan),
                            Rect::from_spans(tracks.index(idx), vspan),
                        )
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw(via)?;
                }
                Ok(())
            };

        draw_vias(&rmux, "read_bl", CellTrack::ReadP)?;
        draw_vias(&rmux, "read_br", CellTrack::ReadN)?;

        draw_vias(&sa, "inp", CellTrack::ReadP)?;
        draw_vias(&sa, "inn", CellTrack::ReadN)?;
        draw_vias(&sa, "outp", CellTrack::ReadP)?;
        draw_vias(&sa, "outn", CellTrack::ReadN)?;

        if self.params.include_wmask {
            draw_vias(&wmux, "wmask", CellTrack::Wmask)?;
        }
        draw_vias(&wmux, "data", CellTrack::Data)?;
        draw_vias(&wmux, "data_b", CellTrack::DataB)?;

        draw_vias(&buf, "inp", CellTrack::ReadP)?;
        draw_vias(&buf, "inn", CellTrack::ReadN)?;
        draw_vias(&buf, "outp", CellTrack::ReadP)?;

        draw_vias(&dff, "d", CellTrack::DataIn)?;
        draw_vias(&dff, "q", CellTrack::Data)?;
        draw_vias(&dff, "qb", CellTrack::DataB)?;

        if self.params.include_wmask {
            // Co-opt the Data track for the wmask input signal
            draw_vias(&wmask_dff, "d", CellTrack::Data)?;
            draw_vias(&wmask_dff, "q", CellTrack::Wmask)?;
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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub enum TapTrack {
    Vdd,
    Vss,
}

impl From<usize> for TapTrack {
    fn from(value: usize) -> Self {
        use TapTrack::*;
        match value {
            0 => Vdd,
            1 => Vss,
            _ => panic!("invalid `TapTrack` index"),
        }
    }
}

impl From<TapTrack> for usize {
    fn from(value: TapTrack) -> usize {
        use TapTrack::*;
        match value {
            Vdd => 0,
            Vss => 1,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub enum CellTrack {
    ReadP,
    ReadN,
    DataIn,
    Data,
    DataB,
    Wmask,
}

impl From<usize> for CellTrack {
    fn from(value: usize) -> Self {
        use CellTrack::*;
        match value {
            0 => ReadP,
            1 => ReadN,
            2 => DataIn,
            3 => Data,
            4 => DataB,
            5 => Wmask,
            _ => panic!("invalid `CellTrack` index"),
        }
    }
}

impl From<CellTrack> for usize {
    fn from(value: CellTrack) -> Self {
        use CellTrack::*;
        match value {
            ReadP => 0,
            ReadN => 1,
            DataIn => 2,
            Data => 3,
            DataB => 4,
            Wmask => 5,
        }
    }
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
        let read_mux_params = ReadMuxParams {
            idx: 0,
            ..self.params.col.rmux.clone()
        };
        // Always use a precharge center tile; the real precharge end
        // is used for the replica and dummy column.
        let mut pc = ctx.instantiate::<PrechargeCent>(&self.params.col.pc)?;
        let (mut rmux, mut wmux) = if self.params.end {
            let rmux = ctx.instantiate::<ReadMuxEnd>(&read_mux_params)?;
            let wmux = ctx.instantiate::<WriteMuxEnd>(&WriteMuxEndParams {
                sizing: self.params.col.wmux,
            })?;
            (rmux, wmux)
        } else {
            let rmux = ctx.instantiate::<ReadMuxCent>(&read_mux_params)?;
            let wmux = ctx.instantiate::<WriteMuxCent>(&WriteMuxCentParams {
                cut_data: true,
                cut_wmask: self.params.cut_wmask,
                sizing: self.params.col.wmux,
            })?;
            (rmux, wmux)
        };
        let mut sa = ctx.instantiate::<SenseAmpCent>(&NoParams)?;
        sa.set_orientation(Named::ReflectVert);
        let mut buf = ctx.instantiate::<DiffBufCent>(&self.params.col.buf)?;
        buf.set_orientation(Named::R90Cw);
        let mut dff = ctx.instantiate::<DffColCent>(&NoParams)?;
        let mut wmask_dff = ctx.instantiate::<DffColCent>(&NoParams)?;
        let mut grid = Grid::new(0, 0);
        grid.push_row(into_vec![pc.clone()]);
        grid.push_row(into_vec![rmux.clone()]);
        grid.push_row(into_vec![wmux.clone()]);
        grid.push_row(into_vec![sa.clone()]);
        grid.push_row(into_vec![buf.clone()]);
        grid.push_row(into_vec![dff.clone()]);
        let wmask_tile = Pad::new(wmask_dff.clone(), DFF_PADDING);
        grid.push_row(into_vec![wmask_tile]);

        let mut tiler = GridTiler::new(grid);
        pc.translate(tiler.translation(0, 0));
        rmux.translate(tiler.translation(1, 0));
        wmux.translate(tiler.translation(2, 0));
        sa.translate(tiler.translation(3, 0));
        buf.translate(tiler.translation(4, 0));
        dff.translate(tiler.translation(5, 0));
        wmask_dff.translate(tiler.translation(6, 0));
        tiler.expose_ports(
            |port: CellPort, (i, _)| match port.name().as_str() {
                "en_b" | "we" | "sel_b" | "vdd" | "vss" => Some(port),
                "clk" => {
                    if i == 3 {
                        Some(port.named("sense_en"))
                    } else {
                        Some(port.with_index(0))
                    }
                }
                _ => None,
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned())?;
        ctx.draw(tiler)?;

        let hspan = Span::new(0, pc.brect().width());
        let tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 330,
            space: 320,
            num: 2,
            span: hspan,
            lower_boundary: Boundary::HalfSpace,
            upper_boundary: Boundary::HalfSpace,
            grid: 5,
        });

        let layers = ctx.layers();
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;
        let vspan = ctx.brect().vspan();

        let track_vspans = |track: TapTrack| -> substrate::error::Result<Vec<Span>> {
            use TapTrack::*;
            Ok(match track {
                Vdd | Vss => vec![vspan],
            })
        };

        for (i, track) in tracks.iter().enumerate() {
            let name = TapTrack::from(i);
            for vspan in track_vspans(name)? {
                let rect = Rect::from_spans(track, vspan);
                let mut port = CellPort::new(match name {
                    TapTrack::Vdd => "vdd",
                    TapTrack::Vss => "vss",
                });
                port.add(m3, subgeom::Shape::Rect(rect));
                ctx.merge_port(port);
                ctx.draw_rect(m3, rect);
            }
        }

        let mut connect =
            |inst: &Instance, port: &str, track: TapTrack| -> substrate::error::Result<()> {
                let idx = track.into();
                let port = inst.port(port)?;
                for shape in port.shapes(m2) {
                    let target_vspan = shape.brect().vspan();
                    let viap = ViaParams::builder()
                        .layers(m2, m3)
                        .geometry(
                            Rect::from_spans(hspan, target_vspan),
                            Rect::from_spans(tracks.index(idx), vspan),
                        )
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw(via)?;
                }
                Ok(())
            };

        connect(&pc, "vdd", TapTrack::Vdd)?;
        connect(&rmux, "vdd", TapTrack::Vdd)?;
        connect(&wmux, "vss", TapTrack::Vss)?;
        connect(&sa, "vdd", TapTrack::Vdd)?;
        connect(&sa, "vss", TapTrack::Vss)?;
        connect(&buf, "vdd", TapTrack::Vdd)?;
        connect(&buf, "vss", TapTrack::Vss)?;
        connect(&dff, "vdd", TapTrack::Vdd)?;
        connect(&dff, "vss", TapTrack::Vss)?;
        connect(&wmask_dff, "vdd", TapTrack::Vdd)?;
        connect(&wmask_dff, "vss", TapTrack::Vss)?;

        Ok(())
    }
}
