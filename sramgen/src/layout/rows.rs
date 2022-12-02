use layout21::raw::translate::Translate;
use layout21::raw::{Instance, Point, TransformTrait};
use pdkprims::Pdk;

use crate::layout::common::sc_outline;

pub struct AlignedRows {
    rows: Vec<Vec<Instance>>,
    grow_right: bool,
    grow_up: bool,
}

impl AlignedRows {
    #[inline]
    pub fn new() -> Self {
        Self {
            rows: vec![],
            grow_right: true,
            grow_up: true,
        }
    }

    pub fn grow_down(&mut self) -> &mut Self {
        self.grow_up = false;
        self
    }

    pub fn grow_left(&mut self) -> &mut Self {
        self.grow_right = false;
        self
    }

    #[inline]
    pub fn add_row(&mut self, row: Vec<Instance>) {
        self.rows.push(row);
    }

    #[inline]
    pub fn place(&mut self, pdk: &Pdk) {
        self.place_inner(pdk)
    }

    pub fn get(&self, row: usize, col: usize) -> &Instance {
        &self.rows[row][col]
    }

    pub fn get_row(&self, row: usize) -> &[Instance] {
        &self.rows[row]
    }

    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut Instance {
        &mut self.rows[row][col]
    }

    fn place_inner(&mut self, pdk: &Pdk) {
        let mut prev_row: Option<Instance> = None;

        for row in self.rows.iter_mut() {
            let mut prev: Option<Instance> = None;
            for (c, instance) in row.iter_mut().enumerate() {
                if let Some(ref p) = prev {
                    let p_outline_bbox = sc_outline(pdk, p).transform(&p.transform());
                    let outline_bbox = sc_outline(pdk, instance).transform(&instance.transform());
                    instance.translate(Point::new(
                        if self.grow_right {
                            p_outline_bbox.p1.x - outline_bbox.p0.x
                        } else {
                            p_outline_bbox.p0.x - outline_bbox.p0.x
                        },
                        p_outline_bbox.p0.y - outline_bbox.p0.y,
                    ));
                } else if let Some(ref p) = prev_row {
                    let p_outline_bbox = sc_outline(pdk, p).transform(&p.transform());
                    let outline_bbox = sc_outline(pdk, instance).transform(&instance.transform());
                    instance.translate(Point::new(
                        p_outline_bbox.p0.x - outline_bbox.p0.x,
                        if self.grow_up {
                            p_outline_bbox.p1.y - outline_bbox.p0.y
                        } else {
                            p_outline_bbox.p0.y - outline_bbox.p1.y
                        },
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
    pub fn rows(&self) -> &Vec<Vec<Instance>> {
        &self.rows
    }

    #[inline]
    pub fn into_instances(self) -> Vec<Instance> {
        self.rows.into_iter().flatten().collect()
    }
}

impl Default for AlignedRows {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
