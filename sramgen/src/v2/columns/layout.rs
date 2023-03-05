use grid::Grid;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{Instance, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::transform::Translate;
use subgeom::{Rect, Span};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::{OptionTile, Pad, Padding, RectBbox, Tile};
use substrate::layout::routing::tracks::{Boundary, CenteredTrackParams, FixedTracks};
use substrate::layout::Draw;

use crate::v2::buf::layout::DiffBufCent;
use crate::v2::buf::DiffBuf;
use crate::v2::columns::Column;
use crate::v2::macros::{DffCol, DffColCent, DffColExtend, SenseAmp, SenseAmpCent};
use crate::v2::precharge::{Precharge, PrechargeCent, PrechargeEnd};
use crate::v2::rmux::{ReadMux, ReadMuxCent, ReadMuxEnd, ReadMuxParams};
use crate::v2::wmux::{
    WriteMux, WriteMuxCent, WriteMuxCentParams, WriteMuxEnd, WriteMuxEndParams, WriteMuxParams,
};

use super::{ColParams, ColPeripherals};

static DFF_PADDING: Padding = Padding::new(160, 0, 0, 0);

impl ColPeripherals {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let mut pc = ctx.instantiate::<Precharge>(&self.params.pc)?;
        let mut pc_end = ctx.instantiate::<PrechargeEnd>(&self.params.pc)?;

        let col = ctx.instantiate::<Column>(&ColParams {
            include_wmask: false,
            ..self.params.clone()
        })?;
        let bbox = Rect::from_spans(Span::new(0, 4_800), col.brect().vspan());
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
        for i in 0..mask_groups {
            for j in 0..self.params.wmask_granularity {
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

        let grid_tiler = GridTiler::new(grid);
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

        ctx.add_port(pc.port("bl_out")?.into_cell_port().named("rbl"));

        pc.orientation_mut().reflect_horiz();
        pc_end.orientation_mut().reflect_horiz();

        pc.align_to_the_right_of(bbox, 0);
        pc.align_top(bbox);
        pc_end.align_to_the_right_of(&pc, 0);
        pc_end.align_top(bbox);

        ctx.draw_ref(&pc)?;
        ctx.draw_ref(&pc_end)?;
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
            row.push(None.into());
        }
        grid.push_row(row);
        let mut row = Vec::new();
        if self.params.include_wmask {
            row.push(OptionTile::new(Tile::from(Pad::new(
                RectBbox::new(dff.clone(), bbox),
                DFF_PADDING,
            ))));
            for _ in 0..mux_ratio - 1 {
                row.push(None.into());
            }
        } else {
            for _ in 0..mux_ratio {
                let cent = ctx.instantiate::<DffColExtend>(&NoParams)?;
                row.push(Pad::new(cent, DFF_PADDING).into());
            }
        }
        grid.push_row(row);

        let tiler = GridTiler::new(grid);
        pc.translate(tiler.translation(0, 0));
        rmux.translate(tiler.translation(1, 0));
        wmux.translate(tiler.translation(2, 0));
        sa.translate(tiler.translation(3, 0));
        buf.translate(tiler.translation(4, 0));
        dff.translate(tiler.translation(5, 0));
        if self.params.include_wmask {
            wmask_dff.translate(tiler.translation(6, 0));
        }
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
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;
        let vspan = ctx.brect().vspan();

        let track_vspans = |track: CellTrack| -> substrate::error::Result<Vec<Span>> {
            use CellTrack::*;
            let pad = 40;
            Ok(match track {
                ReadP | ReadN => vec![
                    Span::new(
                        sa.port("inp")?.largest_rect(m2)?.bottom() - pad,
                        rmux.brect().top(),
                    ),
                    Span::new(
                        buf.port("inp")?.largest_rect(m2)?.bottom() - pad,
                        sa.port("outn")?.largest_rect(m2)?.top() + pad,
                    ),
                    Span::new(
                        vspan.start(),
                        buf.port("outn")?.largest_rect(m2)?.top() + pad,
                    ),
                ],
                Vss => vec![vspan],
                Data | DataB | Wmask => vec![Span::new(vspan.start(), wmux.brect().top())],
            })
        };

        for (i, track) in tracks.iter().enumerate() {
            let name = CellTrack::from(i);
            for vspan in track_vspans(name)? {
                ctx.draw_rect(m3, Rect::from_spans(track, vspan));
            }
        }

        let mut connect =
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

        connect(&rmux, "read_bl", CellTrack::ReadP)?;
        connect(&rmux, "read_br", CellTrack::ReadN)?;

        connect(&sa, "inp", CellTrack::ReadP)?;
        connect(&sa, "inn", CellTrack::ReadN)?;
        connect(&sa, "outp", CellTrack::ReadP)?;
        connect(&sa, "outn", CellTrack::ReadN)?;
        connect(&sa, "vss", CellTrack::Vss)?;

        connect(&wmux, "wmask", CellTrack::Wmask)?;
        connect(&wmux, "data", CellTrack::Data)?;
        connect(&wmux, "data_b", CellTrack::DataB)?;
        connect(&wmux, "vss", CellTrack::Vss)?;

        connect(&buf, "inp", CellTrack::ReadP)?;
        connect(&buf, "inn", CellTrack::ReadN)?;
        connect(&buf, "outp", CellTrack::ReadP)?;
        connect(&buf, "outn", CellTrack::ReadN)?;
        connect(&buf, "vss", CellTrack::Vss)?;

        connect(&dff, "vss", CellTrack::Vss)?;
        connect(&dff, "q", CellTrack::Data)?;
        connect(&dff, "qb", CellTrack::DataB)?;

        if self.params.include_wmask {
            connect(&wmask_dff, "q", CellTrack::Wmask)?;
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
    Vss,
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
            2 => Vss,
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
            Vss => 2,
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

        let tiler = GridTiler::new(grid);
        pc.translate(tiler.translation(0, 0));
        rmux.translate(tiler.translation(1, 0));
        wmux.translate(tiler.translation(2, 0));
        sa.translate(tiler.translation(3, 0));
        buf.translate(tiler.translation(4, 0));
        dff.translate(tiler.translation(5, 0));
        wmask_dff.translate(tiler.translation(6, 0));
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
                ctx.draw_rect(m3, Rect::from_spans(track, vspan));
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
