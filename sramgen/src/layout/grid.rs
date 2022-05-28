use layout21::raw::{Cell, Instance};
use grid::*;

pub struct GridCells {
    grid: Grid<Instance>,
}

impl GridCells {
    pub fn new() -> Self {
        Self {
            grid: grid![],
        }
    }

    pub fn add_row(&mut self, row: Vec<Instance>) {
        self.grid.push_row(row);
    }

    pub fn place(mut self) -> Vec<Instance> {
        self.place_inner()
    }

    fn place_inner(mut self) -> Vec<Instance> {
        let (rows, cols) = self.grid.size();
        let mut insts = Vec::with_capacity(rows*cols);

        let mut prev_row = None;

        for r in 0..rows {
            let mut prev = None;
            for c in 0..self.grid.cols() {
                let mut instance = self.grid.get(r, c).unwrap().clone();
                if let Some(ref p) = prev {
                    instance.align_left_to_right(p).align_bottoms(p);
                } else if let Some(ref p) = prev_row {
                    instance.align_top_to_bottom(p).align_lefts(p);
                }
                insts.push(instance.clone());
                prev = Some(instance.clone());
                if c == 0 {
                    prev_row = Some(instance);
                }
            }
        }

        insts
    }
}
