use grid::Grid;
use substrate::component::Component;
use substrate::layout::context::LayoutCtx;
use substrate::layout::geom::orientation::Named;
use substrate::layout::placement::grid::GridTiler;
use substrate::{into_grid, into_vec};

use crate::v2::precharge::{Precharge, PrechargeCent, PrechargeEnd};
use crate::v2::rmux::{ReadMux, ReadMuxCent, ReadMuxEnd};
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

        let col = into_vec![&pc_end, None, None];
        grid.push_col(col);
        let col = into_vec![&pc, None, None];
        grid.push_col(col);
        let col = into_vec![&pc_cent, &rmux_end, &wmux_end];
        grid.push_col(col);

        for grp in 0..groups {
            for i in 0..mux_ratio {
                let mut pc = pc.clone();
                let mut rmux_params = self.params.rmux.clone();
                rmux_params.idx = i;
                let mut rmux = ctx.instantiate::<ReadMux>(&rmux_params)?;
                let mut wmux = ctx.instantiate::<WriteMux>(&WriteMuxParams {
                    sizing: self.params.wmux,
                    idx: i,
                })?;

                if i % 2 == 1 {
                    rmux.orientation_mut().reflect_horiz();
                    wmux.orientation_mut().reflect_horiz();
                } else {
                    pc.orientation_mut().reflect_horiz();
                }
                let a = into_vec![pc, rmux, wmux];
                grid.push_col(a);
            }

            if grp != groups - 1 {
                let col = into_vec![&pc_cent, &rmux_cent, &wmux_cent];
                grid.push_col(col);
            }
        }

        let col = into_vec![
            &pc_cent,
            rmux_end.with_orientation(Named::ReflectHoriz),
            wmux_end.with_orientation(Named::ReflectHoriz)
        ];
        grid.push_col(col);

        let col = into_vec![pc.with_orientation(Named::ReflectHoriz), None, None];
        grid.push_col(col);
        let col = into_vec![pc_end.with_orientation(Named::ReflectHoriz), None, None];
        grid.push_col(col);

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;
        Ok(())
    }
}
