use crate::error::Result;
use magic_vlsi::{
    cell::{InstanceCell, LayoutCellRef},
    units::{Distance, Vec2},
    MagicInstance,
};

#[derive(Debug, Clone)]
pub struct GridCell {
    cell: LayoutCellRef,
    flip_x: bool,
    flip_y: bool,
}

#[derive(Debug)]
pub struct GridLayout {
    inner: grid::Grid<Option<GridCell>>,
    row_heights: Vec<Distance>,
    col_widths: Vec<Distance>,
}

impl GridCell {
    pub fn new(cell: LayoutCellRef, flip_x: bool, flip_y: bool) -> Self {
        Self {
            cell,
            flip_x,
            flip_y,
        }
    }
}

impl GridLayout {
    #[allow(clippy::needless_range_loop)]
    pub fn new(grid: grid::Grid<Option<GridCell>>) -> Self {
        let (rows, cols) = grid.size();
        let mut row_heights = vec![None; rows];
        let mut col_widths = vec![None; cols];

        for i in 0..rows {
            for j in 0..cols {
                if let Some(grid_cell) = grid.get(i, j).unwrap() {
                    if let Some(h) = row_heights[i] {
                        assert_eq!(grid_cell.cell.bbox.height(), h);
                    } else {
                        row_heights[i] = Some(grid_cell.cell.bbox.height());
                    }

                    if let Some(w) = col_widths[j] {
                        assert_eq!(grid_cell.cell.bbox.width(), w);
                    } else {
                        col_widths[j] = Some(grid_cell.cell.bbox.width());
                    }
                }
            }
        }

        let row_heights = row_heights.into_iter().map(|x| x.unwrap()).collect();
        let col_widths = col_widths.into_iter().map(|x| x.unwrap()).collect();

        Self {
            inner: grid,
            row_heights,
            col_widths,
        }
    }

    pub fn draw(
        &self,
        m: &mut MagicInstance,
        ur: Vec2,
    ) -> Result<grid::Grid<Option<InstanceCell>>> {
        let (rows, cols) = self.inner.size();
        let mut instance_grid = grid::Grid::init(rows, cols, None);

        let mut row_offset = Distance::zero();
        for i in 0..rows {
            let mut col_offset = Distance::zero();
            for j in 0..cols {
                if let Some(cell) = self.inner.get(i, j).unwrap() {
                    let x = ur.x + col_offset;
                    let y = ur.y - row_offset;
                    let mut icell = m.place_layout_cell(cell.cell.clone(), Vec2::new(x, y))?;
                    if cell.flip_x {
                        m.flip_cell_x(&mut icell)?;
                    }
                    if cell.flip_y {
                        m.flip_cell_y(&mut icell)?;
                    }
                    instance_grid[i][j] = Some(icell);
                } else {
                    instance_grid[i][j] = None;
                }
                col_offset += self.col_widths[j];
            }
            row_offset += self.row_heights[i];
        }

        Ok(instance_grid)
    }
}
