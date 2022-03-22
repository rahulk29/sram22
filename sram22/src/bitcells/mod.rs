use log::info;
use magic_vlsi::{units::Vec2, MagicInstance};

use crate::{
    error::Result,
    factory::Component,
    layout::grid::{GridCell, GridLayout},
};

pub struct BitcellArray;

pub struct BitcellArrayParams {
    pub rows: u32,
    pub cols: u32,
}

impl Component for BitcellArray {
    type Params = BitcellArrayParams;

    fn schematic(
        ctx: crate::factory::BuildContext,
        params: Self::Params,
    ) -> micro_hdl::context::ContextTree {
        todo!()
    }
    fn layout(
        mut ctx: crate::factory::BuildContext,
        params: Self::Params,
    ) -> crate::error::Result<crate::factory::Layout> {
        generate_bitcells(ctx.magic, ctx.name, &params)?;
        ctx.layout_from_default_magic()
    }
}

pub(crate) fn generate_bitcells(
    magic: &mut MagicInstance,
    name: &str,
    config: &BitcellArrayParams,
) -> Result<()> {
    info!("generating bitcell array");
    let grid = plan_bitcell_array(magic, config)?;

    magic.load(name)?;
    magic.enable_box()?;
    magic.drc_off()?;
    magic.set_snap(magic_vlsi::SnapMode::Internal)?;

    let grid = GridLayout::new(grid);
    grid.draw(magic, Vec2::zero())?;

    magic.port_renumber()?;
    magic.save(name)?;

    info!("saved {}", name);
    Ok(())
}

pub(crate) fn plan_bitcell_array(
    magic: &mut MagicInstance,
    config: &BitcellArrayParams,
) -> Result<grid::Grid<Option<GridCell>>> {
    let rows = config.rows as usize;

    let top_row = plan_colend_row(magic, config, false)?;

    let bitcell_rows: Result<Vec<Vec<Option<GridCell>>>> = (0..rows as usize)
        .map(|i| {
            info!("planning bitcell row {}", i + 1);
            plan_bitcell_row(magic, config, i)
        })
        .collect();
    let bitcell_rows = bitcell_rows?;

    let bot_row = plan_colend_row(magic, config, true)?;
    let mut grid: grid::Grid<Option<GridCell>> = grid::grid![];
    grid.push_row(top_row);

    for row in bitcell_rows {
        grid.push_row(row);
    }

    grid.push_row(bot_row);

    Ok(grid)
}

pub(crate) fn plan_colend_row(
    magic: &mut MagicInstance,
    config: &BitcellArrayParams,
    bottom: bool,
) -> Result<Vec<Option<GridCell>>> {
    let corner = magic.load_layout_cell("corner")?;
    let colend = magic.load_layout_cell("colend")?;
    let colend_cent = magic.load_layout_cell("colend_cent")?;

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
    magic: &mut MagicInstance,
    config: &BitcellArrayParams,
    idx: usize,
) -> Result<Vec<Option<GridCell>>> {
    let rowend = magic.load_layout_cell("rowend")?;
    let bitcell = magic.load_layout_cell("sram_sp_cell")?;
    let nand2_dec = magic.load_layout_cell("nand2_dec")?;
    let inv_dec = magic.load_layout_cell("inv_dec")?;
    let wlstrap = magic.load_layout_cell("wlstrap")?;

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
