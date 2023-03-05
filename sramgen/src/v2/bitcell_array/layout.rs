use arcstr::ArcStr;

use grid::Grid;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};

use substrate::layout::cell::{CellPort, PortId};
use subgeom::orientation::Named;
use subgeom::Shape;
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;
use substrate::layout::placement::grid::{GridTiler, PortConflictStrategy};
use substrate::layout::placement::nine_patch::{NpTiler, Region};
use substrate::layout::placement::tile::OptionTile;
use substrate::{into_grid, into_vec};

use crate::v2::macros::{
    SpCell, SpCellOpt1a, SpColend, SpColendPCent, SpColenda, SpColendaPCent, SpCorner, SpCornera,
    SpHorizWlstrapP, SpHstrap, SpRowend, SpRowendHstrap, SpRowenda, SpWlstrapP, SpWlstrapaP,
};

use super::SpCellArray;

pub struct SpCellArrayCornerUl;

fn corner_port_map_fn(
    port: CellPort,
    i: usize,
    j: usize,
    edge_i: usize,
    edge_j: usize,
    vmetal: LayerKey,
    hmetal: LayerKey,
) -> Option<CellPort> {
    let mut new_port = CellPort::new(if ["wl", "bl", "br"].contains(&port.name().as_ref()) {
        PortId::from(format!("{}_dummy", port.name()))
    } else {
        port.id().clone()
    });
    if i == edge_i && j == edge_j {
        return Some(port);
    } else if j == edge_j {
        let shapes: Vec<&Shape> = port.shapes(hmetal).collect();

        if !shapes.is_empty() {
            new_port.add_all(hmetal, shapes.into_iter().cloned());
            return Some(new_port);
        }
    } else if i == edge_i {
        let shapes: Vec<&Shape> = port.shapes(vmetal).collect();

        if !shapes.is_empty() {
            new_port.add_all(vmetal, shapes.into_iter().cloned());
            return Some(new_port);
        }
    }
    None
}

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
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        let colend = ctx.instantiate::<SpColend>(&NoParams)?;
        let corner = ctx.instantiate::<SpCorner>(&NoParams)?;
        let rowend = ctx.instantiate::<SpRowend>(&NoParams)?;
        let cell = ctx.instantiate::<SpCell>(&NoParams)?;

        let mut grid_tiler = GridTiler::new(into_grid![[corner, colend][rowend, cell]]);
        grid_tiler.expose_ports(
            |port: CellPort, i, j| corner_port_map_fn(port, i, j, 0, 0, vmetal, hmetal),
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
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
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
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

        let mut grid_tiler =
            GridTiler::new(into_grid![[colend_p_cent, colend, corner][wlstrap_p, cell, rowend]]);
        grid_tiler.expose_ports(
            |port: CellPort, i, j| corner_port_map_fn(port, i, j, 0, 2, vmetal, hmetal),
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
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
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
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

        let mut grid_tiler = GridTiler::new(into_grid![
                    [horiz_wlstrap_p, hstrap, rowend_hstrap]
                    [wlstrap_p, cell, rowend]
                    [colend_p_cent, colend, corner]
        ]);
        grid_tiler.expose_ports(
            |port: CellPort, i, j| corner_port_map_fn(port, i, j, 2, 2, vmetal, hmetal),
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
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
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
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

        let mut grid_tiler = GridTiler::new(into_grid![
                    [rowend_hstrap, hstrap]
                    [rowend, cell]
                    [corner, colend]
        ]);
        grid_tiler.expose_ports(
            |port: CellPort, i, j| corner_port_map_fn(port, i, j, 2, 0, vmetal, hmetal),
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
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
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        let rowend_replica = ctx.instantiate::<SpRowend>(&NoParams)?;
        let mut rowenda_replica = ctx.instantiate::<SpRowenda>(&NoParams)?;
        let mut rowend_hstrap = ctx.instantiate::<SpRowendHstrap>(&NoParams)?;
        let cell = ctx.instantiate::<SpCell>(&NoParams)?;
        let mut cell_opt1a = ctx.instantiate::<SpCellOpt1a>(&NoParams)?;
        let mut hstrap = ctx.instantiate::<SpHstrap>(&NoParams)?;
        rowenda_replica.set_orientation(Named::ReflectVert);
        cell_opt1a.set_orientation(Named::ReflectVert);
        rowend_hstrap.set_orientation(Named::ReflectVert);
        hstrap.set_orientation(Named::ReflectHoriz);

        let cell_row: Vec<OptionTile> = into_vec![rowend_replica, cell];
        let cell_opt1a_row: Vec<OptionTile> = into_vec![rowenda_replica, cell_opt1a];
        let hstrap: Vec<OptionTile> = into_vec![rowend_hstrap, hstrap];

        let mut grid = Grid::new(0, 0);
        grid.push_row(hstrap);
        for _ in 0..self.params.hstrap_ratio / 2 {
            grid.push_row(cell_opt1a_row.clone());
            grid.push_row(cell_row.clone());
        }

        let mut grid_tiler = GridTiler::new(grid);
        grid_tiler.expose_ports(
            |port: CellPort, i, j| {
                let mut new_port = CellPort::new(if port.name() == "wl" {
                    PortId::new("wl", i - 1)
                } else {
                    port.id().clone()
                });
                if j == 0 {
                    let shapes: Vec<&Shape> = port.shapes(hmetal).collect();

                    if !shapes.is_empty() {
                        new_port.add_all(hmetal, shapes.into_iter().cloned());
                        return Some(new_port);
                    }
                }
                None
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
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

        let mut grid_tiler = GridTiler::new(grid);
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        grid_tiler.expose_ports(
            |port: CellPort, i, j| {
                let mut new_port = CellPort::new(if ["bl", "br"].contains(&port.name().as_ref()) {
                    PortId::new(port.name(), j - 1)
                } else {
                    port.id().clone()
                });
                if i == 0 {
                    let shapes: Vec<&Shape> = port.shapes(vmetal).collect();
                    if !shapes.is_empty() {
                        new_port.add_all(vmetal, shapes.into_iter().cloned());
                        return Some(new_port);
                    }
                }
                None
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
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

        let mut grid_tiler = GridTiler::new(grid);
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        grid_tiler.expose_ports(
            |port: CellPort, i, j| {
                let mut new_port = CellPort::new(if ["bl", "br"].contains(&port.name().as_ref()) {
                    PortId::new(port.name(), j - 1)
                } else {
                    port.id().clone()
                });
                if i == 2 {
                    let shapes: Vec<&Shape> = port.shapes(vmetal).collect();

                    if !shapes.is_empty() {
                        new_port.add_all(vmetal, shapes.into_iter().cloned());
                        return Some(new_port);
                    }
                }
                None
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
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
        let mut rowend = ctx.instantiate::<SpRowend>(&NoParams)?;
        let mut rowenda = ctx.instantiate::<SpRowenda>(&NoParams)?;
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
        let hstrap_ratio = 8;
        let nx = self.params.cols / self.params.mux_ratio;
        let ny = self.params.rows / hstrap_ratio;

        let tap_ratio = TapRatio {
            mux_ratio: self.params.mux_ratio,
            hstrap_ratio,
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
            .nx(nx)
            .ny(ny)
            .build();

        let mut grid_tiler = tiler.into_grid_tiler();
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        grid_tiler.expose_ports(
            |mut port: CellPort, i, j| {
                if i == 0 && j == 0 {
                    return Some(port);
                }
                if i == ny + 1 && j == 0 {
                    if port.name() == "wl_dummy" {
                        port.set_id(PortId::new("wl_dummy", 1));
                    }
                    return Some(port);
                }
                if i == ny + 1 && j == nx + 1 {
                    if ["wl_dummy", "bl_dummy", "br_dummy"].contains(&port.name().as_ref()) {
                        port.set_id(PortId::new(port.name(), 1));
                    }
                    return Some(port);
                }
                let mut new_port = CellPort::new(if ["bl", "br"].contains(&port.name().as_ref()) {
                    PortId::new(
                        port.name(),
                        self.params.mux_ratio * (j - 1) + port.id().index(),
                    )
                } else if port.name() == "wl" {
                    PortId::new(port.name(), hstrap_ratio * (i - 1) + port.id().index())
                } else {
                    port.id().clone()
                });

                if j == 0 {
                    let shapes: Vec<&Shape> = port.shapes(hmetal).collect();

                    if !shapes.is_empty() {
                        new_port.add_all(hmetal, shapes.into_iter().cloned());
                        return Some(new_port);
                    }
                } else if i == 0 {
                    if !["bl", "br"].contains(&port.name().as_ref()) {
                        let shapes: Vec<&Shape> = port.shapes(vmetal).collect();

                        if !shapes.is_empty() {
                            new_port.add_all(vmetal, shapes.into_iter().cloned());
                            return Some(new_port);
                        }
                    }
                } else if i == ny + 1 {
                    let shapes: Vec<&Shape> = port.shapes(vmetal).collect();

                    if !shapes.is_empty() {
                        new_port.add_all(vmetal, shapes.into_iter().cloned());
                        return Some(new_port);
                    }
                }
                None
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned());
        ctx.draw(grid_tiler)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::paths::out_gds;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::{SpCellArrayBottom, TapRatio};

    #[test]
    #[ignore]
    fn test_sp_cell_array_bottom() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array_bottom");
        let tap_ratio = TapRatio {
            mux_ratio: 4,
            hstrap_ratio: 8,
        };
        ctx.write_layout::<SpCellArrayBottom>(&tap_ratio, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
    }
}
