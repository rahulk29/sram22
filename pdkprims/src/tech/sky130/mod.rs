use layout21::raw::{
    BoundBoxTrait, Cell, Element, Instance, LayerPurpose, Layout, LayoutResult, Point, Rect, Shape,
};
use layout21::utils::Ptr;

use crate::config::Int;
use crate::contact::ContactParams;
use crate::geometry::{expand_box, CoarseDirection};
use crate::mos::MosType;
use crate::{
    config::TechConfig,
    mos::{MosError, MosParams, MosResult},
    Pdk,
};

#[cfg(test)]
mod tests;

const SKY130_DRC_CONFIG_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tech/sky130/drc_config.toml"
));

fn tech_config() -> TechConfig {
    TechConfig::from_toml(SKY130_DRC_CONFIG_TOML).expect("failed to load sky130A tech config")
}

pub fn pdk() -> LayoutResult<Pdk> {
    Pdk::new(tech_config())
}

impl Pdk {
    pub fn draw_sky130_mos(&self, params: MosParams) -> MosResult<Ptr<Cell>> {
        params.validate()?;

        let mut elems = Vec::new();
        let mut insts = Vec::new();

        let tc = self.config.read().unwrap();
        let layers = self.layers.read().unwrap();

        let poly = layers.keyname("poly").unwrap();
        let diff = layers.keyname("diff").unwrap();

        let nf = params.fingers();

        // Diff length perpendicular to gates
        let diff_perp =
            2 * diff_edge_to_gate(&tc) + nf * tc.layer("poly").width + (nf - 1) * finger_space(&tc);

        let mut prev = None;
        let x0 = 0;
        let mut cx = x0;
        let y0 = 0;

        let mut diff_xs = Vec::new();

        for d in params.devices.iter() {
            if let Some(mt) = prev {
                if mt != d.mos_type {
                    cx += diff_to_opposite_diff(&tc);
                } else {
                    cx += tc.layer("diff").space;
                }
            }

            diff_xs.push(cx);

            let rect = Rect {
                p0: Point::new(cx, y0),
                p1: Point::new(cx + d.width, y0 + diff_perp),
            };

            if d.mos_type == MosType::Pmos {
                let mut psdm_box = rect.clone();
                expand_box(&mut psdm_box, tc.layer("diff").enclosure("psdm"));
                elems.push(Element {
                    net: None,
                    layer: layers.keyname("psdm").unwrap(),
                    purpose: LayerPurpose::Drawing,
                    inner: Shape::Rect(psdm_box),
                });

                let mut well_box = rect.clone();
                expand_box(&mut well_box, tc.layer("diff").enclosure("nwell"));

                elems.push(Element {
                    net: None,
                    layer: layers.keyname("nwell").unwrap(),
                    purpose: LayerPurpose::Drawing,
                    inner: Shape::Rect(well_box),
                });
            } else {
                let mut nsdm_box = rect.clone();
                expand_box(&mut nsdm_box, tc.layer("diff").enclosure("nsdm"));
                elems.push(Element {
                    net: None,
                    layer: layers.keyname("nsdm").unwrap(),
                    purpose: LayerPurpose::Drawing,
                    inner: Shape::Rect(nsdm_box),
                });
            }

            elems.push(Element {
                net: None,
                layer: diff,
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(rect),
            });

            cx += d.width;

            prev = Some(d.mos_type);
        }

        let xpoly = x0 - tc.layer("poly").extension("diff");
        let mut ypoly = y0 + diff_edge_to_gate(&tc);
        let wpoly = cx - xpoly + tc.layer("poly").extension("diff");
        for _ in 0..nf {
            let rect = Rect {
                p0: Point::new(xpoly, ypoly),
                p1: Point::new(xpoly + wpoly, ypoly + params.length()),
            };

            elems.push(Element {
                net: None,
                layer: poly,
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(rect),
            });

            ypoly += params.length();
            ypoly += finger_space(&tc);
        }

        // Add source/drain contacts
        let mut cy = y0;

        for i in 0..=nf {
            for (d, (j, x)) in params.devices.iter().zip(diff_xs.iter().enumerate()) {
                if d.skip_sd_metal.contains(&(i as usize)) {
                    continue;
                }
                let ct = self.get_contact_sized("diffc", diff, d.width).unwrap();
                let bbox = ct.bboxes.get(&diff).unwrap();
                let ofsx = (d.width - rect_width(bbox)) / 2;
                let inst = Instance {
                    inst_name: format!("sd_contact_{}_{}", i, j),
                    cell: Ptr::clone(&ct.cell),
                    loc: Point::new(x - bbox.p0.x + ofsx, cy - bbox.p0.y),
                    reflect_vert: false,
                    angle: None,
                };
                insts.push(inst);
            }
            cy += params.length();
            cy += finger_space(&tc);
        }

        let layout = Layout {
            name: "ptx".to_string(),
            insts,
            annotations: vec![],
            elems,
        };

        let cell = Cell {
            name: "ptx".to_string(),
            abs: None,
            layout: Some(layout),
        };

        Ok(Ptr::new(cell))
    }
}

pub fn finger_space(tc: &TechConfig) -> Int {
    [
        2 * tc.space("gate", "licon") + tc.layer("li").width,
        tc.layer("poly").space,
    ]
    .into_iter()
    .max()
    .unwrap()
}

pub fn diff_edge_to_gate(tc: &TechConfig) -> Int {
    [
        tc.layer("diff").extension("poly"),
        tc.space("gate", "licon") + tc.layer("licon").width + tc.layer("licon").enclosure("diff"),
    ]
    .into_iter()
    .max()
    .unwrap()
}

pub fn diff_to_opposite_diff(tc: &TechConfig) -> Int {
    tc.space("diff", "nwell") + tc.layer("diff").enclosure("nwell")
}

/// Calculates the width of the given rectangle.
///
/// Assumes that `r.p1.x > r.p0.x`.
fn rect_width(r: &Rect) -> Int {
    r.p1.x - r.p0.x
}
