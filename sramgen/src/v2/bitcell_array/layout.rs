use std::path::PathBuf;

use arcstr::ArcStr;
use codegen::hard_macro;
use grid::Grid;
use substrate::component::{Component, NoParams, View};
use substrate::data::SubstrateCtx;
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::{Point, Rect};
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::nine_patch::{NpTiler, Region};
use substrate::layout::placement::tile::{OptionTile, RelativeRectBbox};
use substrate::{into_grid, into_vec};

use super::{SpCell, SpCellArray, SpCellReplica, SpColend};
use crate::tech::external_gds_path;

fn layout_path(_ctx: &SubstrateCtx, name: &str, view: View) -> Option<PathBuf> {
    match view {
        View::Layout => Some(external_gds_path().join(format!("{}.gds", name))),
        _ => None,
    }
}

#[hard_macro(
    name = "sram_sp_colend_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend_cent"
)]
pub struct SpColendCent;

#[hard_macro(
    name = "sram_sp_colend_p_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend_p_cent"
)]
pub struct SpColendPCent;

#[hard_macro(
    name = "sram_sp_corner",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_corner"
)]
pub struct SpCorner;

#[hard_macro(
    name = "sram_sp_hstrap",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_hstrap"
)]
pub struct SpHstrap;

#[hard_macro(
    name = "sram_sp_rowend",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend"
)]
pub struct SpRowend;

#[hard_macro(
    name = "sram_sp_rowend_hstrap",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend"
)]
pub struct SpRowendHstrap;

#[hard_macro(
    name = "sram_sp_rowend_replica",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_rowend_replica"
)]
pub struct SpRowendReplica;

#[hard_macro(
    name = "sram_sp_wlstrap",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrap"
)]
pub struct SpWlstrap;

#[hard_macro(
    name = "sram_sp_wlstrap_p",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrap_p"
)]
pub struct SpWlstrapP;

#[hard_macro(
    name = "sram_sp_horiz_wlstrap_p",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_horiz_wlstrap_p"
)]
pub struct SpHorizWlstrapP;

#[hard_macro(
    name = "sram_sp_cell_opt1a",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cell_opt1a",
    spice_subckt_name = "sky130_fd_bd_sram__sram_sp_cell_opt1a"
)]
pub struct SpCellOpt1a;

#[hard_macro(
    name = "sram_sp_cell_opt1a_replica",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_cell_opt1a_replica"
)]
pub struct SpCellOpt1aReplica;

#[hard_macro(
    name = "sram_sp_colenda",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda"
)]
pub struct SpColenda;

#[hard_macro(
    name = "sram_sp_colenda_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda_cent"
)]
pub struct SpColendaCent;

#[hard_macro(
    name = "sram_sp_colenda_p_cent",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda_p_cent"
)]
pub struct SpColendaPCent;

#[hard_macro(
    name = "sram_sp_cornera",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cornera"
)]
pub struct SpCornera;

#[hard_macro(
    name = "sram_sp_rowenda",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowenda"
)]
pub struct SpRowenda;

#[hard_macro(
    name = "sram_sp_rowenda_replica",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_rowenda_replica"
)]
pub struct SpRowendaReplica;

#[hard_macro(
    name = "sram_sp_wlstrapa",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrapa"
)]
pub struct SpWlstrapa;

#[hard_macro(
    name = "sram_sp_wlstrapa_p",
    pdk = "sky130-open",
    path_fn = "layout_path",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrapa_p"
)]
pub struct SpWlstrapaP;

pub struct SpCellArrayCornerTop;

impl Component for SpCellArrayCornerTop {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_corner_top")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let colend = ctx.instantiate::<SpColend>(&NoParams)?;
        let corner = ctx.instantiate::<SpCorner>(&NoParams)?;
        let rowend = ctx.instantiate::<SpRowend>(&NoParams)?;
        let cell = ctx.instantiate::<SpCell>(&NoParams)?;

        let grid_tiler = GridTiler::new(into_grid![[corner, colend][rowend, cell]]);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayLeft;

impl Component for SpCellArrayLeft {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_left")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let rowend_replica = ctx.instantiate::<SpRowendReplica>(&NoParams)?;
        let mut rowenda_replica = ctx.instantiate::<SpRowendaReplica>(&NoParams)?;
        let mut rowend_hstrap = ctx.instantiate::<SpRowendHstrap>(&NoParams)?;
        let cell_replica = ctx.instantiate::<SpCellReplica>(&NoParams)?;
        let mut cell_opt1a_replica = ctx.instantiate::<SpCellOpt1aReplica>(&NoParams)?;
        let mut hstrap = ctx.instantiate::<SpHstrap>(&NoParams)?;
        rowenda_replica.set_orientation(Named::ReflectVert);
        cell_opt1a_replica.set_orientation(Named::ReflectVert);
        rowend_hstrap.set_orientation(Named::ReflectVert);
        hstrap.set_orientation(Named::ReflectHoriz);

        let replica_bbox = Rect::new(Point::new(70, 0), Point::new(1270, 1580));

        let cell_replica_tile = RelativeRectBbox::new(cell_replica, replica_bbox);
        let cell_opt1a_replica_tile = RelativeRectBbox::new(cell_opt1a_replica, replica_bbox);

        let cell_row: Vec<OptionTile> = into_vec![rowend_replica, cell_replica_tile];
        let cell_opt1a_row: Vec<OptionTile> = into_vec![rowenda_replica, cell_opt1a_replica_tile];
        let hstrap: Vec<OptionTile> = into_vec![rowend_hstrap, hstrap];

        let mut grid = Grid::new(0, 0);
        for _ in 0..2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }
        grid.push_row(hstrap);
        for _ in 0..2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayCornerBottom;

impl Component for SpCellArrayCornerBottom {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_corner_bottom")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let mut colenda = ctx.instantiate::<SpColenda>(&NoParams)?;
        let mut cornera = ctx.instantiate::<SpCornera>(&NoParams)?;
        let mut rowenda = ctx.instantiate::<SpRowenda>(&NoParams)?;
        let mut cell_opt1a = ctx.instantiate::<SpCellOpt1a>(&NoParams)?;
        colenda.set_orientation(Named::ReflectVert);
        cornera.set_orientation(Named::ReflectVert);
        rowenda.set_orientation(Named::ReflectVert);
        cell_opt1a.set_orientation(Named::ReflectVert);

        let grid_tiler = GridTiler::new(into_grid![[rowenda, cell_opt1a][cornera, colenda]]);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayTop;

impl Component for SpCellArrayTop {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_top")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let colend_1 = ctx.instantiate::<SpColend>(&NoParams)?;
        let mut colend_2 = ctx.instantiate::<SpColend>(&NoParams)?;
        let cell_1 = ctx.instantiate::<SpCell>(&NoParams)?;
        let mut cell_2 = ctx.instantiate::<SpCell>(&NoParams)?;
        let wlstrap_p = ctx.instantiate::<SpWlstrapP>(&NoParams)?;
        let colend_p_cent = ctx.instantiate::<SpColendPCent>(&NoParams)?;
        colend_2.set_orientation(Named::ReflectHoriz);
        cell_2.set_orientation(Named::ReflectHoriz);

        let cell_1_col = into_vec![colend_1, cell_1];
        let cell_2_col = into_vec![colend_2, cell_2];
        let wlstrap = into_vec![colend_p_cent, wlstrap_p];

        let mut grid = Grid::new(0, 0);
        for _ in 0..2 {
            grid.push_col(cell_2_col.clone());
            grid.push_col(cell_1_col.clone());
        }
        grid.push_col(wlstrap);
        for _ in 0..2 {
            grid.push_col(cell_2_col.clone());
            grid.push_col(cell_1_col.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayCenter;

impl Component for SpCellArrayCenter {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_center")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let cell_1 = ctx.instantiate::<SpCell>(&NoParams)?;
        let mut cell_2 = ctx.instantiate::<SpCell>(&NoParams)?;
        let mut cell_opt1a_1 = ctx.instantiate::<SpCellOpt1a>(&NoParams)?;
        let mut cell_opt1a_2 = ctx.instantiate::<SpCellOpt1a>(&NoParams)?;
        let wlstrap_p = ctx.instantiate::<SpWlstrapP>(&NoParams)?;
        let mut wlstrapa_p = ctx.instantiate::<SpWlstrapaP>(&NoParams)?;
        let mut hstrap_1 = ctx.instantiate::<SpHstrap>(&NoParams)?;
        let hstrap_2 = ctx.instantiate::<SpHstrap>(&NoParams)?;
        let horiz_wlstrap_p = ctx.instantiate::<SpHorizWlstrapP>(&NoParams)?;

        cell_2.set_orientation(Named::ReflectHoriz);
        cell_opt1a_1.set_orientation(Named::ReflectVert);
        cell_opt1a_2.set_orientation(Named::R180);
        wlstrapa_p.set_orientation(Named::ReflectVert);
        hstrap_1.set_orientation(Named::ReflectHoriz);

        let cell_row = into_vec![
            &cell_2, &cell_1, &cell_2, &cell_1, &wlstrap_p, &cell_2, &cell_1, &cell_2, &cell_1
        ];
        let cell_opt1a_row = into_vec![
            &cell_opt1a_2,
            &cell_opt1a_1,
            &cell_opt1a_2,
            &cell_opt1a_1,
            &wlstrapa_p,
            &cell_opt1a_2,
            &cell_opt1a_1,
            &cell_opt1a_2,
            &cell_opt1a_1
        ];
        let hstrap = into_vec![
            &hstrap_2,
            &hstrap_1,
            &hstrap_2,
            &hstrap_1,
            &horiz_wlstrap_p,
            &hstrap_2,
            &hstrap_1,
            &hstrap_2,
            &hstrap_1
        ];

        let mut grid = Grid::new(0, 0);
        for _ in 0..2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }
        grid.push_row(hstrap);
        for _ in 0..2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayBottom;

impl Component for SpCellArrayBottom {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_top")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let mut colenda_1 = ctx.instantiate::<SpColenda>(&NoParams)?;
        let mut colenda_2 = ctx.instantiate::<SpColenda>(&NoParams)?;
        let mut cell_opt1a_1 = ctx.instantiate::<SpCellOpt1a>(&NoParams)?;
        let mut cell_opt1a_2 = ctx.instantiate::<SpCellOpt1a>(&NoParams)?;
        let mut wlstrapa_p = ctx.instantiate::<SpWlstrapaP>(&NoParams)?;
        let mut colenda_p_cent = ctx.instantiate::<SpColendaPCent>(&NoParams)?;
        colenda_1.set_orientation(Named::ReflectVert);
        colenda_2.set_orientation(Named::R180);
        cell_opt1a_1.set_orientation(Named::ReflectVert);
        cell_opt1a_2.set_orientation(Named::R180);
        wlstrapa_p.set_orientation(Named::ReflectVert);
        colenda_p_cent.set_orientation(Named::ReflectVert);

        let cell_1_col = into_vec![cell_opt1a_1, colenda_1];
        let cell_2_col = into_vec![cell_opt1a_2, colenda_2];
        let wlstrap = into_vec![wlstrapa_p, colenda_p_cent];

        let mut grid = Grid::new(0, 0);
        for _ in 0..2 {
            grid.push_col(cell_2_col.clone());
            grid.push_col(cell_1_col.clone());
        }
        grid.push_col(wlstrap);
        for _ in 0..2 {
            grid.push_col(cell_2_col.clone());
            grid.push_col(cell_1_col.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayRight;

impl Component for SpCellArrayRight {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_right")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let mut rowend = ctx.instantiate::<SpRowendReplica>(&NoParams)?;
        let mut rowenda = ctx.instantiate::<SpRowendaReplica>(&NoParams)?;
        let mut rowend_hstrap = ctx.instantiate::<SpRowendHstrap>(&NoParams)?;
        let mut cell = ctx.instantiate::<SpCell>(&NoParams)?;
        let mut cell_opt1a = ctx.instantiate::<SpCellOpt1a>(&NoParams)?;
        let hstrap = ctx.instantiate::<SpHstrap>(&NoParams)?;
        rowend.set_orientation(Named::ReflectHoriz);
        rowenda.set_orientation(Named::R180);
        rowend_hstrap.set_orientation(Named::R180);
        cell.set_orientation(Named::ReflectHoriz);
        cell_opt1a.set_orientation(Named::R180);

        let cell_row: Vec<OptionTile> = into_vec![cell, rowend];
        let cell_opt1a_row = into_vec![cell_opt1a, rowenda];
        let hstrap = into_vec![hstrap, rowend_hstrap];

        let mut grid = Grid::new(0, 0);
        for _ in 0..2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }
        grid.push_row(hstrap);
        for _ in 0..2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

impl SpCellArray {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let cell_array_corner_upper_left = ctx.instantiate::<SpCellArrayCornerTop>(&NoParams)?;
        let cell_array_left = ctx.instantiate::<SpCellArrayLeft>(&NoParams)?;
        let cell_array_corner_lower_left = ctx.instantiate::<SpCellArrayCornerBottom>(&NoParams)?;

        let cell_array_top = ctx.instantiate::<SpCellArrayTop>(&NoParams)?;
        let cell_array_center = ctx.instantiate::<SpCellArrayCenter>(&NoParams)?;
        let cell_array_bottom = ctx.instantiate::<SpCellArrayBottom>(&NoParams)?;

        let mut cell_array_corner_upper_right =
            ctx.instantiate::<SpCellArrayCornerTop>(&NoParams)?;
        let cell_array_right = ctx.instantiate::<SpCellArrayRight>(&NoParams)?;
        let mut cell_array_corner_lower_right =
            ctx.instantiate::<SpCellArrayCornerBottom>(&NoParams)?;
        cell_array_corner_upper_right.set_orientation(Named::ReflectHoriz);
        cell_array_corner_lower_right.set_orientation(Named::ReflectHoriz);

        let tiler = NpTiler::builder()
            .set(Region::CornerUl, &cell_array_corner_upper_left)
            .set(Region::Left, &cell_array_left)
            .set(Region::CornerLl, &cell_array_corner_lower_left)
            .set(Region::Top, &cell_array_top)
            .set(Region::Center, &cell_array_center)
            .set(Region::Bottom, &cell_array_bottom)
            .set(Region::CornerUr, &cell_array_corner_upper_right)
            .set(Region::Right, &cell_array_right)
            .set(Region::CornerLr, &cell_array_corner_lower_right)
            .nx(self.params.cols / 8)
            .ny(self.params.rows / 8)
            .build();

        ctx.draw(tiler)?;
        Ok(())
    }
}
