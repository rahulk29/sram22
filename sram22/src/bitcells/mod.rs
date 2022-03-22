use log::info;
use magic_vlsi::units::Vec2;

use crate::{
    error::Result,
    factory::{BuildContext, Component},
    layout::grid::{GridCell, GridLayout},
    names::{
        ARRAY_COLEND, ARRAY_COLEND_CENTER, ARRAY_CORNER, INV_DEC, NAND2_DEC, ROWEND, SP_BITCELL,
        WLSTRAP,
    },
};

pub struct BitcellArray;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BitcellArrayParams {
    pub rows: u32,
    pub cols: u32,
}

impl Component for BitcellArray {
    type Params = BitcellArrayParams;

    fn schematic(
        _ctx: crate::factory::BuildContext,
        _params: Self::Params,
    ) -> micro_hdl::context::ContextTree {
        todo!()
    }
    fn layout(
        mut ctx: crate::factory::BuildContext,
        params: Self::Params,
    ) -> crate::error::Result<crate::factory::Layout> {
        generate_bitcells(&mut ctx, &params)?;
        ctx.layout_from_default_magic()
    }
}

pub(crate) fn generate_bitcells(ctx: &mut BuildContext, config: &BitcellArrayParams) -> Result<()> {
    info!("generating bitcell array");
    let grid = plan_bitcell_array(ctx, config)?;
    let magic = &mut ctx.magic;

    magic.load(ctx.name)?;
    magic.enable_box()?;
    magic.drc_off()?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    let grid = GridLayout::new(grid);
    grid.draw(magic, Vec2::zero())?;

    magic.port_renumber()?;
    magic.save(ctx.name)?;
    Ok(())
}

pub(crate) fn plan_bitcell_array(
    ctx: &mut BuildContext,
    config: &BitcellArrayParams,
) -> Result<grid::Grid<Option<GridCell>>> {
    let rows = config.rows as usize;

    let top_row = plan_colend_row(ctx, config, false)?;

    let bitcell_rows: Result<Vec<Vec<Option<GridCell>>>> = (0..rows as usize)
        .map(|i| {
            info!("planning bitcell row {}", i + 1);
            plan_bitcell_row(ctx, config, i)
        })
        .collect();
    let bitcell_rows = bitcell_rows?;

    let bot_row = plan_colend_row(ctx, config, true)?;
    let mut grid: grid::Grid<Option<GridCell>> = grid::grid![];
    grid.push_row(top_row);

    for row in bitcell_rows {
        grid.push_row(row);
    }

    grid.push_row(bot_row);

    Ok(grid)
}

pub(crate) fn plan_colend_row(
    ctx: &mut BuildContext,
    config: &BitcellArrayParams,
    bottom: bool,
) -> Result<Vec<Option<GridCell>>> {
    let corner = ctx.factory.require_layout(ARRAY_CORNER)?.cell;
    let colend = ctx.factory.require_layout(ARRAY_COLEND)?.cell;
    let colend_cent = ctx.factory.require_layout(ARRAY_COLEND_CENTER)?.cell;

    // 2 slots for decoder gates
    let mut top_row = vec![
        None,
        None,
        Some(GridCell::new(corner.clone(), true, bottom)),
    ];

    for i in 0..config.cols as usize {
        if i > 0 && i % 8 == 0 {
            top_row.push(Some(GridCell::new(colend_cent.clone(), i % 2 != 0, bottom)));
        }
        top_row.push(Some(GridCell::new(colend.clone(), i % 2 != 0, bottom)));
    }

    top_row.push(Some(GridCell::new(corner, false, bottom)));

    info!("generated {} row cells", top_row.len());

    Ok(top_row)
}

pub(crate) fn plan_bitcell_row(
    ctx: &mut BuildContext,
    config: &BitcellArrayParams,
    idx: usize,
) -> Result<Vec<Option<GridCell>>> {
    let rowend = ctx.factory.require_layout(ROWEND)?.cell;
    let bitcell = ctx.factory.require_layout(SP_BITCELL)?.cell;
    let nand2_dec = ctx.factory.require_layout(NAND2_DEC)?.cell;
    let inv_dec = ctx.factory.require_layout(INV_DEC)?.cell;
    let wlstrap = ctx.factory.require_layout(WLSTRAP)?.cell;

    let mut row = Vec::new();
    let flip_y = idx % 2 == 0;

    row.push(Some(GridCell::new(nand2_dec, false, flip_y)));
    row.push(Some(GridCell::new(inv_dec, false, flip_y)));
    row.push(Some(GridCell::new(rowend.clone(), true, flip_y)));

    for i in 0..config.cols as usize {
        if i > 0 && i % 8 == 0 {
            row.push(Some(GridCell::new(wlstrap.clone(), false, flip_y)));
        }
        row.push(Some(GridCell::new(bitcell.clone(), i % 2 == 0, flip_y)));
    }

    row.push(Some(GridCell::new(rowend, false, flip_y)));

    Ok(row)
}
