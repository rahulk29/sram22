use grid::*;
use layout21::raw::align::AlignRect;
use layout21::raw::{BoundBoxTrait, Instance};

pub struct GridCells {
    grid: Grid<Instance>,
}

impl GridCells {
    #[inline]
    pub fn new() -> Self {
        Self { grid: grid![] }
    }

    #[inline]
    pub fn add_row(&mut self, row: Vec<Instance>) {
        self.grid.push_row(row);
    }

    #[inline]
    pub fn place(&mut self) {
        self.place_inner()
    }

    fn place_inner(&mut self) {
        let (rows, cols) = self.grid.size();
        let mut prev_row: Option<Instance> = None;

        for r in 0..rows {
            let mut prev: Option<Instance> = None;
            for c in 0..cols {
                let instance = self.grid.get_mut(r, c).unwrap();
                if let Some(ref p) = prev {
                    instance.align_to_the_right_of(p.bbox(), 0);
                    instance.align_bottom(p.bbox());
                } else if let Some(ref p) = prev_row {
                    instance.align_beneath(p.bbox(), 0);
                    instance.align_left(p.bbox());
                }
                prev = Some(instance.clone());
                if c == 0 {
                    prev_row = Some(instance.clone());
                }
            }
        }
    }

    #[inline]
    pub fn grid(&self) -> &Grid<Instance> {
        &self.grid
    }

    #[inline]
    pub fn into_instances(self) -> Vec<Instance> {
        self.grid.into_vec()
    }
}

impl Default for GridCells {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
