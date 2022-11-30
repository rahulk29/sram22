use grid::*;
use layout21::raw::align::{AlignMode, AlignRect};
use layout21::raw::TransformTrait;
use layout21::raw::{translate::Translate, BoundBoxTrait, Instance, Int, Point};
use pdkprims::Pdk;

use crate::layout::common::sc_outline;

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
    pub fn place(&mut self, pdk: &Pdk) {
        self.place_inner(pdk)
    }

    fn place_inner(&mut self, pdk: &Pdk) {
        let (rows, cols) = self.grid.size();
        let mut prev_row: Option<Instance> = None;

        for r in 0..rows {
            let mut prev: Option<Instance> = None;
            for c in 0..cols {
                let instance = self.grid.get_mut(r, c).unwrap();
                if let Some(ref p) = prev {
                    let p_outline_bbox = sc_outline(pdk, p).transform(&p.transform());
                    let outline_bbox = sc_outline(pdk, instance).transform(&instance.transform());
                    instance.translate(Point::new(
                        p_outline_bbox.p1.x - outline_bbox.p0.x,
                        p_outline_bbox.p0.y - outline_bbox.p0.y,
                    ));
                } else if let Some(ref p) = prev_row {
                    let p_outline_bbox = sc_outline(pdk, p).transform(&p.transform());
                    let outline_bbox = sc_outline(pdk, instance).transform(&instance.transform());
                    instance.translate(Point::new(
                        p_outline_bbox.p0.x - outline_bbox.p0.x,
                        p_outline_bbox.p0.y - outline_bbox.p1.y,
                    ));
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
