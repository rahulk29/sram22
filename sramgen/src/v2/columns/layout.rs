use grid::Grid;
use substrate::component::{Component, NoParams};
use substrate::into_vec;
use substrate::layout::context::LayoutCtx;
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::Rect;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::tile::{RectBbox, Tile};

use crate::v2::bitcell_array::SenseAmp;
use crate::v2::precharge::{Precharge, PrechargeCent, PrechargeEnd};
use crate::v2::rmux::{ReadMux, ReadMuxCent, ReadMuxEnd};
use crate::v2::wmux::{
    WriteMux, WriteMuxCent, WriteMuxCentParams, WriteMuxEnd, WriteMuxEndParams, WriteMuxParams,
};

use super::ColPeripherals;

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
                    let sa = sa.clone();
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
