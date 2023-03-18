use crate::v2::macros::{SpCellOpt1aReplica, SpCellReplica, SpColend, SpCorner, SpRowendReplica};
use arcstr::ArcStr;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use subgeom::orientation::Named;
use subgeom::Shape;
use substrate::component::{Component, NoParams};
use substrate::into_grid;

use substrate::layout::cell::{CellPort, PortConflictStrategy, PortId};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::nine_patch::{NpTiler, Region};
use substrate::layout::placement::tile::LayerBbox;
use substrate::schematic::circuit::Direction;

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
        let mut grid_tiler = GridTiler::new(grid);
        let vmetal = ctx.layers().get(Selector::Metal(1))?;
        grid_tiler.expose_ports(
            |port: CellPort, (i, j)| {
                let mut new_port = CellPort::new(if port.name() == "wl" {
                    PortId::new("wl", i)
                } else {
                    port.id().clone()
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
        ctx.add_ports(grid_tiler.ports().cloned());
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
        arcstr::literal!("replica_cell_array_rowend")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let rowend = ctx.instantiate::<SpRowendReplica>(&NoParams)?;
        let rowend_flip = rowend.with_orientation(Named::ReflectVert);

        let grid = into_grid![[rowend][rowend_flip]];
        let mut grid_tiler = GridTiler::new(grid);
        let hmetal = ctx.layers().get(Selector::Metal(2))?;
        grid_tiler.expose_ports(
            |port: CellPort, (i, j)| {
                let mut new_port = CellPort::new(if port.name() == "wl" {
                    PortId::new("wl", i)
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
        ctx.add_ports(grid_tiler.ports().cloned());
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

        for i in 0..self.params.rows {
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

        Ok(())
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

        let nx = self.params.cols / 2;
        let ny = self.params.rows / 2;

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
                let mut new_port = CellPort::new(if ["bl", "br"].contains(&port.name().as_ref()) {
                    PortId::new(port.name(), 2 * (j - 1) + port.id().index())
                } else if port.name() == "wl" {
                    PortId::new(port.name(), 2 * (i - 1) + port.id().index())
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
                    if port.name() != "wl" {
                        let shapes: Vec<&Shape> = port.shapes(hmetal).collect();

                        if !shapes.is_empty() {
                            new_port.add_all(hmetal, shapes.into_iter().cloned());
                            return Some(new_port);
                        }
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
