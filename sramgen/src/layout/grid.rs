use grid::*;
use layout21::raw::align::AlignRect;
use layout21::raw::{BoundBoxTrait, Instance};

pub struct GridCells {
    grid: Grid<Instance>,
}

impl GridCells {
    pub fn new() -> Self {
        Self { grid: grid![] }
    }

    pub fn add_row(&mut self, row: Vec<Instance>) {
        self.grid.push_row(row);
    }

    pub fn place(self) -> Vec<Instance> {
        self.place_inner()
    }

    fn place_inner(self) -> Vec<Instance> {
        let (rows, cols) = self.grid.size();
        let mut insts = Vec::with_capacity(rows * cols);

        let mut prev_row: Option<Instance> = None;

        for r in 0..rows {
            let mut prev: Option<Instance> = None;
            for c in 0..self.grid.cols() {
                let mut instance = self.grid.get(r, c).unwrap().clone();
                if let Some(ref p) = prev {
                    instance.align_to_the_right_of(p.bbox(), 0);
                    instance.align_beneath(p.bbox(), 0);
                } else if let Some(ref p) = prev_row {
                    instance.align_beneath(p.bbox(), 0);
                    instance.align_left(p.bbox());
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

impl Default for GridCells {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
