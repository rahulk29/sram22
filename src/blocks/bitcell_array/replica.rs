use crate::blocks::macros::{
    SpCellOpt1aReplica, SpCellReplica, SpColend, SpColenda, SpCorner, SpCornera, SpHstrap,
    SpRowendHstrap, SpRowendReplica, SpRowendaReplica, SpRowtapendReplica,
};
use arcstr::ArcStr;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Shape, Span};
use substrate::component::{Component, NoParams};
use substrate::into_grid;

use substrate::layout::cell::{CellPort, PortConflictStrategy, PortId};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::nine_patch::{NpTiler, Region};
use substrate::layout::placement::tile::{LayerBbox, RectBbox};
use substrate::schematic::circuit::Direction;

pub struct ReplicaCellArray {
    params: ReplicaCellArrayParams,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct ReplicaCellArrayParams {
    pub rows: usize,
    pub cols: usize,
}

pub struct RowTapEndRowendHstrap;

impl Component for RowTapEndRowendHstrap {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_wlstrap_rowend_hstrap")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let wlstrap = ctx
            .instantiate::<SpRowtapendReplica>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let rowend = ctx
            .instantiate::<SpRowendHstrap>(&NoParams)?
            .with_orientation(Named::ReflectVert);

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let (a, b) = [wlstrap, rowend]
            .into_iter()
            .map(|cell| LayerBbox::new(cell, outline))
            .collect_tuple()
            .unwrap();

        let grid = into_grid![[a, b]];
        let mut grid = GridTiler::new(grid);
        grid.expose_ports(
            |port: CellPort, (_i, j)| {
                if j == 1 {
                    Some(port)
                } else {
                    None
                }
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid.ports().cloned()).unwrap();
        ctx.draw(grid)?;

        Ok(())
    }
}

pub struct Center;

impl Component for Center {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
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
        let replica_a_flip = replica_a.with_orientation(Named::R180);

        let hstrap = ctx.instantiate::<SpHstrap>(&NoParams)?;
        let hstrap_flip = hstrap.with_orientation(Named::ReflectHoriz);

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let (a, b, c, d, e, f) = [
            replica,
            replica_flip,
            replica_a,
            replica_a_flip,
            hstrap,
            hstrap_flip,
        ]
        .into_iter()
        .map(|cell| LayerBbox::new(cell, outline))
        .collect_tuple()
        .unwrap();

        let grid = into_grid![[f, e][c.clone(), d.clone()][a.clone(), b.clone()][c, d][a, b]];
        let grid = GridTiler::new(grid);
        ctx.draw(grid)?;

        Ok(())
    }
}

pub struct Top;

impl Component for Top {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_cell_array_colend_top")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let colend = ctx.instantiate::<SpColend>(&NoParams)?;
        let colend_flip = colend.with_orientation(Named::ReflectHoriz);

        let replica = ctx.instantiate::<SpCellReplica>(&NoParams)?;
        let replica_a = ctx
            .instantiate::<SpCellOpt1aReplica>(&NoParams)?
            .with_orientation(Named::ReflectVert);

        let replica_flip = replica.with_orientation(Named::ReflectHoriz);
        let replica_a_flip = replica.with_orientation(Named::R180);

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let (a, b, _c, _d) = [replica, replica_flip, replica_a, replica_a_flip]
            .into_iter()
            .map(|cell| LayerBbox::new(cell, outline))
            .collect_tuple()
            .unwrap();

        let grid = into_grid![[colend, colend_flip][a, b]];
        let mut grid_tiler = GridTiler::new(grid);
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        grid_tiler.expose_ports(
            |port: CellPort, (i, j)| {
                let mut new_port = CellPort::new(match port.name().as_str() {
                    "wl" => PortId::new("wl", i),
                    "bl" | "br" => PortId::new(port.name(), j),
                    _ => port.id().clone(),
                });
                let shapes: Vec<&Shape> = port.shapes(vmetal).collect();

                if !shapes.is_empty() {
                    new_port.add_all(vmetal, shapes.into_iter().cloned());
                    return Some(new_port);
                }
                None
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned()).unwrap();
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct Bot;

impl Component for Bot {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_cell_array_colend_top")
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
        let replica_a_flip = replica_a.with_orientation(Named::R180);

        let hstrap = ctx.instantiate::<SpHstrap>(&NoParams)?;
        let hstrap_flip = hstrap.with_orientation(Named::ReflectHoriz);

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let (_a, _b, c, d, e, f) = [
            replica,
            replica_flip,
            replica_a,
            replica_a_flip,
            hstrap,
            hstrap_flip,
        ]
        .into_iter()
        .map(|cell| LayerBbox::new(cell, outline))
        .collect_tuple()
        .unwrap();

        let colend = ctx
            .instantiate::<SpColenda>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let colend_flip = colend.with_orientation(Named::R180);

        let grid = into_grid![[f, e][c, d][colend, colend_flip]];
        let mut grid_tiler = GridTiler::new(grid);
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        grid_tiler.expose_ports(
            |port: CellPort, (i, j)| {
                let mut new_port = CellPort::new(match port.name().as_str() {
                    "wl" => PortId::new("wl", i),
                    "bl" | "br" => PortId::new(port.name(), j),
                    _ => port.id().clone(),
                });
                let shapes: Vec<&Shape> = port.shapes(vmetal).collect();

                if !shapes.is_empty() {
                    new_port.add_all(vmetal, shapes.into_iter().cloned());
                    return Some(new_port);
                }
                None
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned()).unwrap();
        ctx.draw(grid_tiler)?;

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
        arcstr::literal!("replica_cell_array_leftright")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let rowend = ctx.instantiate::<SpRowendReplica>(&NoParams)?;
        let rowenda = ctx
            .instantiate::<SpRowendaReplica>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let rowend_hstrap = ctx.instantiate::<RowTapEndRowendHstrap>(&NoParams)?;
        let rowend_hstrap_bbox = rowend_hstrap.bbox().into_rect();
        let rowend_hstrap = RectBbox::new(
            rowend_hstrap,
            rowend_hstrap_bbox.with_hspan(Span::with_stop_and_length(
                rowend_hstrap_bbox.right(),
                1_300,
            )),
        );

        let grid = into_grid![[rowend_hstrap][rowenda.clone()][rowend.clone()][rowenda][rowend]];
        let mut grid_tiler = GridTiler::new(grid);
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        grid_tiler.expose_ports(
            |port: CellPort, (i, _j)| {
                let mut new_port = CellPort::new(if port.name() == "wl" {
                    PortId::new("wl", i - 1)
                } else {
                    port.id().clone()
                });
                let shapes: Vec<&Shape> = port.shapes(hmetal).collect();

                if !shapes.is_empty() {
                    new_port.add_all(hmetal, shapes.into_iter().cloned());
                    return Some(new_port);
                }
                None
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned()).unwrap();
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct CornerTop;

impl Component for CornerTop {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_cell_array_corner_top")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let rowend = ctx.instantiate::<SpRowendReplica>(&NoParams)?;
        let corner = ctx.instantiate::<SpCorner>(&NoParams)?;

        let grid = into_grid![[corner][rowend]];
        let mut grid_tiler = GridTiler::new(grid);
        grid_tiler.expose_ports(
            |port: CellPort, (i, _j)| {
                let new_id = if port.name() == "wl" {
                    PortId::new("wl", i - 1)
                } else {
                    port.id().clone()
                };
                Some(port.with_id(new_id))
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned()).unwrap();
        ctx.draw(grid_tiler)?;

        Ok(())
    }
}

pub struct CornerBot;

impl Component for CornerBot {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("replica_cell_array_corner_bot")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let rowend = ctx
            .instantiate::<SpRowendaReplica>(&NoParams)?
            .with_orientation(Named::ReflectVert);
        let corner = ctx
            .instantiate::<SpCornera>(&NoParams)?
            .with_orientation(Named::ReflectVert);

        let rowend_hstrap = ctx.instantiate::<RowTapEndRowendHstrap>(&NoParams)?;
        let rowend_hstrap_bbox = rowend_hstrap.bbox().into_rect();
        let rowend_hstrap = RectBbox::new(
            rowend_hstrap,
            rowend_hstrap_bbox.with_hspan(Span::with_stop_and_length(
                rowend_hstrap_bbox.right(),
                1_300,
            )),
        );

        let grid = into_grid![[rowend_hstrap][rowend][corner]];
        let mut grid_tiler = GridTiler::new(grid);
        grid_tiler.expose_ports(
            |port: CellPort, (i, _j)| {
                let new_id = if port.name() == "wl" {
                    PortId::new("wl", i - 1)
                } else {
                    port.id().clone()
                };
                Some(port.with_id(new_id))
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(grid_tiler.ports().cloned()).unwrap();
        ctx.draw(grid_tiler)?;

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

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let bl = ctx.port("rbl", Direction::InOut);
        let br = ctx.port("rbr", Direction::InOut);
        let wl = ctx.port("rwl", Direction::Input);

        let rows = (self.params.rows - 2).div_ceil(4) * 4 + 2;

        for i in 0..rows {
            for j in 0..self.params.cols {
                let wl = if i == 0 { wl } else { vss };
                ctx.instantiate::<SpCellReplica>(&NoParams)?
                    .with_connections([
                        ("BL", bl),
                        ("BR", br),
                        ("VSS", vss),
                        ("VDD", vdd),
                        ("VPB", vdd),
                        ("VNB", vss),
                        ("WL", wl),
                    ])
                    .named(format!("cell_{i}_{j}"))
                    .add_to(ctx);
            }
        }

        for j in 0..self.params.cols {
            for i in 0..2 {
                ctx.instantiate::<SpColend>(&NoParams)?
                    .with_connections([
                        ("BL", bl),
                        ("BR", br),
                        ("VSS", vss),
                        ("VDD", vdd),
                        ("VPB", vdd),
                        ("VNB", vss),
                    ])
                    .named(format!("colend_{i}_{j}"))
                    .add_to(ctx);
            }
        }

        let rowtaps = (rows - 2).div_ceil(4) + 1;
        for j in 0..rowtaps {
            for i in 0..2 {
                ctx.instantiate::<SpRowtapendReplica>(&NoParams)?
                    .with_connections([("VSS", vss), ("VNB", vss)])
                    .named(format!("rowtapend_{j}_{i}"))
                    .add_to(ctx);
            }
            for i in 0..self.params.cols {
                ctx.instantiate::<SpHstrap>(&NoParams)?
                    .with_connections([
                        ("BL", bl),
                        ("BR", br),
                        ("VDD", vdd),
                        ("VPB", vdd),
                        ("VSS", vss),
                        ("VNB", vss),
                    ])
                    .named(format!("hstrap_{j}_{i}"))
                    .add_to(ctx);
            }
        }

        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let corner_ul = ctx.instantiate::<CornerTop>(&NoParams)?;
        let left = ctx.instantiate::<LeftRight>(&NoParams)?;
        let left_bbox = left.bbox().into_rect();
        let left = RectBbox::new(
            left,
            left_bbox.with_hspan(Span::with_stop_and_length(left_bbox.right(), 1_300)),
        );

        let corner_ll = ctx.instantiate::<CornerBot>(&NoParams)?;

        let top = ctx.instantiate::<Top>(&NoParams)?;
        let center = ctx.instantiate::<Center>(&NoParams)?;
        let bot = ctx.instantiate::<Bot>(&NoParams)?;

        let corner_ur = corner_ul.clone().with_orientation(Named::ReflectHoriz);
        let right = ctx
            .instantiate::<LeftRight>(&NoParams)?
            .with_orientation(Named::ReflectHoriz);
        let right_bbox = right.bbox().into_rect();
        let right = RectBbox::new(
            right,
            right_bbox.with_hspan(Span::with_start_and_length(right_bbox.left(), 1_300)),
        );
        let corner_lr = corner_ll.clone().with_orientation(Named::ReflectHoriz);
        let corner_ll_bbox = corner_ll.bbox().into_rect();
        let corner_ll = RectBbox::new(
            corner_ll,
            corner_ll_bbox.with_hspan(Span::with_stop_and_length(corner_ll_bbox.right(), 1_300)),
        );
        let corner_lr_bbox = corner_lr.bbox().into_rect();
        let corner_lr = RectBbox::new(
            corner_lr,
            corner_lr_bbox.with_hspan(Span::with_start_and_length(corner_lr_bbox.left(), 1_300)),
        );

        let nx = self.params.cols / 2;
        let ny = (self.params.rows - 2).div_ceil(4);

        let tiler = NpTiler::builder()
            .set(Region::CornerUl, LayerBbox::new(corner_ul, outline))
            .set(Region::Left, left)
            .set(Region::CornerLl, corner_ll)
            .set(Region::Top, LayerBbox::new(top, outline))
            .set(Region::Center, LayerBbox::new(center, outline))
            .set(Region::Bottom, LayerBbox::new(bot, outline))
            .set(Region::CornerUr, LayerBbox::new(corner_ur, outline))
            .set(Region::Right, right)
            .set(Region::CornerLr, corner_lr)
            .nx(nx)
            .ny(ny)
            .build();

        let mut grid_tiler = tiler.into_grid_tiler();
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        grid_tiler.expose_ports(
            |port: CellPort, (i, j)| {
                if (i == 0 || i == ny + 1) && (j == 0 || j == nx + 1) {
                    return Some(port);
                }
                let mut new_port = CellPort::new(if port.name() == "wl" {
                    PortId::new(port.name(), 4 * (i - 1) + port.id().index())
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
                } else if j == nx + 1 {
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
        ctx.add_ports(grid_tiler.ports().cloned()).unwrap();
        ctx.draw(grid_tiler)?;

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
