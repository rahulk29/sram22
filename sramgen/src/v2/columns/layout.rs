use grid::Grid;
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::into_vec;
use substrate::layout::cell::{Instance, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::transform::Translate;
use substrate::layout::geom::{Rect, Span};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::{OptionTile, RectBbox, Tile};
use substrate::layout::routing::tracks::{Boundary, CenteredTrackParams, FixedTracks};

use crate::v2::bitcell_array::SenseAmp;
use crate::v2::precharge::{Precharge, PrechargeCent, PrechargeEnd};
use crate::v2::rmux::{ReadMux, ReadMuxCent, ReadMuxEnd, ReadMuxParams};
use crate::v2::wmux::{
    WriteMux, WriteMuxCent, WriteMuxCentParams, WriteMuxEnd, WriteMuxEndParams, WriteMuxParams,
};

use super::{ColParams, ColPeripherals};

impl ColPeripherals {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let mux_ratio = self.params.rmux.mux_ratio;
        let groups = 16;

        let pc_end = ctx.instantiate::<PrechargeEnd>(&self.params.pc)?;
        let pc = ctx.instantiate::<Precharge>(&self.params.pc)?;
        let pc_cent = ctx.instantiate::<PrechargeCent>(&self.params.pc)?;

        let rmux_end = ctx.instantiate::<ReadMuxEnd>(&self.params.rmux)?;
        let rmux_cent = ctx.instantiate::<ReadMuxCent>(&self.params.rmux)?;

        let wmux_end = ctx.instantiate::<WriteMuxEnd>(&WriteMuxEndParams {
            sizing: self.params.wmux,
        })?;
        let wmux_cent = ctx.instantiate::<WriteMuxCent>(&WriteMuxCentParams {
            sizing: self.params.wmux,
            cut_data: true,
            cut_wmask: true,
        })?;

        let mut grid = Grid::new(0, 0);

        let col = into_vec![&pc_end, None, None, None];
        grid.push_col(col);
        let col = into_vec![&pc, None, None, None];
        grid.push_col(col);
        let col = into_vec![&pc_cent, &rmux_end, &wmux_end, None];
        grid.push_col(col);

        for grp in 0..groups {
            let sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
            for i in 0..mux_ratio {
                let mut pc = pc.clone();
                let mut rmux_params = self.params.rmux.clone();
                rmux_params.idx = i;
                let mut rmux = ctx.instantiate::<ReadMux>(&rmux_params)?;
                let mut wmux = ctx.instantiate::<WriteMux>(&WriteMuxParams {
                    sizing: self.params.wmux,
                    idx: i,
                })?;

                let sa = if i == 0 {
                    let sa = sa.with_orientation(Named::ReflectVert);
                    let bbox = Rect::from_spans(pc.brect().hspan(), sa.brect().vspan());
                    Some(Tile::from(RectBbox::new(sa, bbox)))
                } else {
                    None
                };

                if i % 2 == 1 {
                    rmux.orientation_mut().reflect_horiz();
                    wmux.orientation_mut().reflect_horiz();
                } else {
                    pc.orientation_mut().reflect_horiz();
                }
                let a = into_vec![pc, rmux, wmux, sa];
                grid.push_col(a);
            }

            if grp != groups - 1 {
                let col = into_vec![&pc_cent, &rmux_cent, &wmux_cent, None];
                grid.push_col(col);
            }
        }

        let col = into_vec![
            &pc_cent,
            rmux_end.with_orientation(Named::ReflectHoriz),
            wmux_end.with_orientation(Named::ReflectHoriz),
            None
        ];
        grid.push_col(col);

        let col = into_vec![pc.with_orientation(Named::ReflectHoriz), None, None, None];
        grid.push_col(col);
        let col = into_vec![
            pc_end.with_orientation(Named::ReflectHoriz),
            None,
            None,
            None
        ];
        grid.push_col(col);

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;
        Ok(())
    }
}

pub struct Column {
    params: ColParams,
}

impl Component for Column {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
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
        for i in 0..mux_ratio / 2 {
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

        let tiler = GridTiler::new(grid);
        pc.translate(tiler.translation(0, 0));
        rmux.translate(tiler.translation(1, 0));
        wmux.translate(tiler.translation(2, 0));
        sa.translate(tiler.translation(3, 0));
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

        for track in tracks.iter() {
            ctx.draw_rect(m3, Rect::from_spans(track, vspan));
        }

        let mut connect =
            |inst: &Instance, port: &str, track: usize| -> substrate::error::Result<()> {
                let target_vspan = inst.port(port)?.largest_rect(m2)?.vspan();
                let viap = ViaParams::builder()
                    .layers(m2, m3)
                    .geometry(
                        Rect::from_spans(hspan, target_vspan),
                        Rect::from_spans(tracks.index(track), vspan),
                    )
                    .build();
                let via = ctx.instantiate::<Via>(&viap)?;
                ctx.draw(via)?;
                Ok(())
            };

        connect(&rmux, "read_bl", 0)?;
        connect(&sa, "inp", 0)?;
        connect(&rmux, "read_br", 1)?;
        connect(&sa, "inn", 1)?;

        Ok(())
    }
}
