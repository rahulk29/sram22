use std::path::PathBuf;

use arcstr::ArcStr;
use codegen::hard_macro;
use grid::Grid;
use serde::{Deserialize, Serialize};
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
        View::Layout => Some(external_gds_path().join(format!("{name}.gds"))),
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

pub struct SpCellArrayCornerUl;

impl Component for SpCellArrayCornerUl {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_corner_ul")
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

pub struct SpCellArrayCornerUr;

impl Component for SpCellArrayCornerUr {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_corner_ur")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let colend = ctx
            .instantiate::<SpColend>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let corner = ctx
            .instantiate::<SpCorner>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let rowend = ctx
            .instantiate::<SpRowend>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let colend_p_cent = ctx.instantiate::<SpColendPCent>(&NoParams)?;
        let wlstrap_p = ctx.instantiate::<SpWlstrapP>(&NoParams)?;
        let cell = ctx
            .instantiate::<SpCell>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);

        let grid_tiler =
            GridTiler::new(into_grid![[colend_p_cent, colend, corner][wlstrap_p, cell, rowend]]);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayCornerLr;

impl Component for SpCellArrayCornerLr {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_corner_lr")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let colend = ctx
            .instantiate::<SpColend>(&NoParams)?
            .with_orientation(Named::R180);
        let corner = ctx
            .instantiate::<SpCorner>(&NoParams)?
            .with_orientation(Named::R180);
        let rowend = ctx
            .instantiate::<SpRowend>(&NoParams)?
            .with_orientation(Named::R180);
        let horiz_wlstrap_p = ctx.instantiate::<SpHorizWlstrapP>(&NoParams)?;
        let colend_p_cent = ctx
            .instantiate::<SpColendPCent>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let wlstrap_p = ctx
            .instantiate::<SpWlstrapP>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let hstrap = ctx.instantiate::<SpHstrap>(&NoParams)?;
        let rowend_hstrap = ctx
            .instantiate::<SpRowendHstrap>(&NoParams)?
            .with_orientation(Named::R180);
        let cell = ctx
            .instantiate::<SpCell>(&NoParams)?
            .with_orientation(Named::R180);

        let grid_tiler = GridTiler::new(into_grid![
                    [horiz_wlstrap_p, hstrap, rowend_hstrap]
                    [wlstrap_p, cell, rowend]
                    [colend_p_cent, colend, corner]
        ]);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayCornerLl;

impl Component for SpCellArrayCornerLl {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_corner_ll")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let colend = ctx
            .instantiate::<SpColend>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let corner = ctx
            .instantiate::<SpCorner>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let rowend = ctx
            .instantiate::<SpRowend>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let hstrap = ctx
            .instantiate::<SpHstrap>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let rowend_hstrap = ctx
            .instantiate::<SpRowendHstrap>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let cell = ctx
            .instantiate::<SpCell>(&NoParams)?
            .with_orientation(Named::ReflectVert);

        let grid_tiler = GridTiler::new(into_grid![
                    [rowend_hstrap, hstrap]
                    [rowend, cell]
                    [corner, colend]
        ]);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayLeft {
    params: TapRatio,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TapRatio {
    pub mux_ratio: usize,
    pub hstrap_ratio: usize,
}

impl Component for SpCellArrayLeft {
    type Params = TapRatio;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
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
        grid.push_row(hstrap);
        for _ in 0..self.params.hstrap_ratio / 2 {
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

pub struct SpCellArrayTop {
    params: TapRatio,
}

impl Component for SpCellArrayTop {
    type Params = TapRatio;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
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
        grid.push_col(wlstrap);
        for _ in 0..self.params.mux_ratio / 2 {
            grid.push_col(cell_2_col.clone());
            grid.push_col(cell_1_col.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayCenter {
    params: TapRatio,
}

impl Component for SpCellArrayCenter {
    type Params = TapRatio;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
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

        let mut cell_row = Vec::new();
        let mut cell_opt1a_row = Vec::new();
        let mut hstrap_row = Vec::new();

        cell_row.push(wlstrap_p.into());
        hstrap_row.push(horiz_wlstrap_p.into());
        cell_opt1a_row.push(wlstrapa_p.clone().into());
        for _ in 0..self.params.mux_ratio / 2 {
            cell_row.push(cell_2.clone().into());
            cell_row.push(cell_1.clone().into());
            cell_opt1a_row.push(cell_opt1a_2.clone().into());
            cell_opt1a_row.push(cell_opt1a_1.clone().into());
            hstrap_row.push(hstrap_2.clone().into());
            hstrap_row.push(hstrap_1.clone().into());
        }

        let mut grid = Grid::new(0, 0);
        grid.push_row(hstrap_row);
        for _ in 0..self.params.hstrap_ratio / 2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayBottom {
    params: TapRatio,
}

impl Component for SpCellArrayBottom {
    type Params = TapRatio;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array_bot")
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
        let hstrap_1 = ctx
            .instantiate::<SpHstrap>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let hstrap_2 = ctx.instantiate::<SpHstrap>(&NoParams)?;
        let horiz_wlstrap_p = ctx.instantiate::<SpHorizWlstrapP>(&NoParams)?;
        colenda_1.set_orientation(Named::ReflectVert);
        colenda_2.set_orientation(Named::R180);
        cell_opt1a_1.set_orientation(Named::ReflectVert);
        cell_opt1a_2.set_orientation(Named::R180);
        wlstrapa_p.set_orientation(Named::ReflectVert);
        colenda_p_cent.set_orientation(Named::ReflectVert);

        let cell_1_col = into_vec![hstrap_1, cell_opt1a_1, colenda_1];
        let cell_2_col = into_vec![hstrap_2, cell_opt1a_2, colenda_2];
        let wlstrap = into_vec![horiz_wlstrap_p, wlstrapa_p, colenda_p_cent];

        let mut grid = Grid::new(0, 0);
        grid.push_col(wlstrap);
        for _ in 0..self.params.mux_ratio / 2 {
            grid.push_col(cell_2_col.clone());
            grid.push_col(cell_1_col.clone());
        }

        let grid_tiler = GridTiler::new(grid);
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct SpCellArrayRight {
    params: TapRatio,
}

impl Component for SpCellArrayRight {
    type Params = TapRatio;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
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
        let horiz_wlstrap_p = ctx.instantiate::<SpHorizWlstrapP>(&NoParams)?;
        let wlstrap_p = ctx.instantiate::<SpWlstrapP>(&NoParams)?;
        let wlstrapa_p = ctx
            .instantiate::<SpWlstrapaP>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        rowend.set_orientation(Named::ReflectHoriz);
        rowenda.set_orientation(Named::R180);
        rowend_hstrap.set_orientation(Named::R180);
        cell.set_orientation(Named::ReflectHoriz);
        cell_opt1a.set_orientation(Named::R180);

        let cell_row: Vec<OptionTile> = into_vec![wlstrap_p, cell, rowend];
        let cell_opt1a_row = into_vec![wlstrapa_p, cell_opt1a, rowenda];
        let hstrap = into_vec![horiz_wlstrap_p, hstrap, rowend_hstrap];

        let mut grid = Grid::new(0, 0);
        grid.push_row(hstrap);
        for _ in 0..self.params.hstrap_ratio / 2 {
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
        let tap_ratio = TapRatio {
            mux_ratio: self.params.mux_ratio,
            hstrap_ratio: 8,
        };
        let corner_ul = ctx.instantiate::<SpCellArrayCornerUl>(&NoParams)?;
        let left = ctx.instantiate::<SpCellArrayLeft>(&tap_ratio)?;
        let corner_ll = ctx.instantiate::<SpCellArrayCornerLl>(&NoParams)?;

        let top = ctx.instantiate::<SpCellArrayTop>(&tap_ratio)?;
        let center = ctx.instantiate::<SpCellArrayCenter>(&tap_ratio)?;
        let bot = ctx.instantiate::<SpCellArrayBottom>(&tap_ratio)?;

        let corner_ur = ctx.instantiate::<SpCellArrayCornerUr>(&NoParams)?;
        let right = ctx.instantiate::<SpCellArrayRight>(&tap_ratio)?;
        let corner_lr = ctx.instantiate::<SpCellArrayCornerLr>(&NoParams)?;

        let tiler = NpTiler::builder()
            .set(Region::CornerUl, &corner_ul)
            .set(Region::Left, &left)
            .set(Region::CornerLl, &corner_ll)
            .set(Region::Top, &top)
            .set(Region::Center, &center)
            .set(Region::Bottom, &bot)
            .set(Region::CornerUr, &corner_ur)
            .set(Region::Right, &right)
            .set(Region::CornerLr, &corner_lr)
            .nx(self.params.cols / self.params.mux_ratio)
            .ny(self.params.rows / 8)
            .build();

        ctx.draw(tiler)?;
        Ok(())
    }
}
