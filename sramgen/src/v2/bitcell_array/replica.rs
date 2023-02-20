use crate::v2::macros::{SpCellOpt1aReplica, SpCellReplica, SpColend, SpCorner, SpRowendReplica};
use arcstr::ArcStr;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::into_grid;
use substrate::layout::geom::orientation::Named;

use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::nine_patch::{NpTiler, Region};
use substrate::layout::placement::tile::LayerBbox;

use super::layout::TapRatio;

pub struct ReplicaCellArray {
    params: ReplicaCellArrayParams,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct ReplicaCellArrayParams {
    pub rows: usize,
    pub cols: usize,
}

pub struct Center {
    params: TapRatio,
}

impl Component for Center {
    type Params = TapRatio;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_cell_array_center")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let replica = ctx.instantiate::<SpCellReplica>(&NoParams)?;
        let replica_a = ctx
            .instantiate::<SpCellOpt1aReplica>(&NoParams)?
            .with_orientation(Named::ReflectVert);

        let replica_flip = replica.with_orientation(Named::ReflectHoriz);
        let replica_a_flip = replica.with_orientation(Named::R180);

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let (a, b, c, d) = [replica, replica_flip, replica_a, replica_a_flip]
            .into_iter()
            .map(|cell| LayerBbox::new(cell, outline))
            .collect_tuple()
            .unwrap();

        let grid = into_grid![[a, b][c, d]];
        let grid = GridTiler::new(grid);
        ctx.draw(grid)?;

        Ok(())
    }
}

pub struct TopBot;

impl Component for TopBot {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_cell_array_colend")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let colend = ctx.instantiate::<SpColend>(&NoParams)?;
        let colend_flip = colend.with_orientation(Named::ReflectHoriz);

        let grid = into_grid![[colend, colend_flip]];
        let grid = GridTiler::new(grid);
        ctx.draw(grid)?;

        Ok(())
    }
}

pub struct LeftRight;

impl Component for LeftRight {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_cell_array_rowend")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let rowend = ctx.instantiate::<SpRowendReplica>(&NoParams)?;
        let rowend_flip = rowend.with_orientation(Named::ReflectVert);

        let grid = into_grid![[rowend][rowend_flip]];
        let grid = GridTiler::new(grid);
        ctx.draw(grid)?;

        Ok(())
    }
}

impl Component for ReplicaCellArray {
    type Params = ReplicaCellArrayParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("replica_cell_array")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let tap_ratio = TapRatio {
            mux_ratio: 2,
            hstrap_ratio: 8,
        };

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let corner_ul = ctx.instantiate::<SpCorner>(&NoParams)?;
        let left = ctx.instantiate::<LeftRight>(&NoParams)?;
        let corner_ll = corner_ul.clone().with_orientation(Named::ReflectVert);

        let top = ctx.instantiate::<TopBot>(&NoParams)?;
        let center = ctx.instantiate::<Center>(&tap_ratio)?;
        let bot = ctx
            .instantiate::<TopBot>(&NoParams)?
            .with_orientation(Named::ReflectVert);

        let corner_ur = corner_ul.clone().with_orientation(Named::ReflectHoriz);
        let right = ctx
            .instantiate::<LeftRight>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let corner_lr = corner_ul.clone().with_orientation(Named::R180);

        let tiler = NpTiler::builder()
            .set(Region::CornerUl, &corner_ul)
            .set(Region::Left, &left)
            .set(Region::CornerLl, &corner_ll)
            .set(Region::Top, &top)
            .set(Region::Center, LayerBbox::new(center, outline))
            .set(Region::Bottom, &bot)
            .set(Region::CornerUr, &corner_ur)
            .set(Region::Right, &right)
            .set(Region::CornerLr, &corner_lr)
            .nx(self.params.cols / 2)
            .ny(self.params.rows / 2)
            .build();

        ctx.draw(tiler)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::out_gds;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    fn test_replica_cell_array() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_replica_cell_array");
        ctx.write_layout::<ReplicaCellArray>(
            &ReplicaCellArrayParams { rows: 24, cols: 2 },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }
}
