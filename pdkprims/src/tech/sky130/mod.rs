use std::collections::HashMap;
use std::sync::Arc;

use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Element, Instance, LayerKey, LayerPurpose, Layout,
    LayoutResult, Library, Point, Rect, Shape, Units,
};
use layout21::utils::Ptr;

use crate::config::{Int, Uint};
use crate::{LayerIdx, PdkLib, Ref};

use crate::contact::{Contact, ContactParams};
use crate::geometry::{
    expand_box, expand_box_min_width, rect_from_bbox, translate, CoarseDirection,
};
use crate::mos::{LayoutTransistors, MosType};
use crate::{
    config::TechConfig,
    mos::{MosParams, MosResult},
    Pdk,
};

use self::layers::Sky130Pdk;

pub(crate) mod layers;
#[cfg(test)]
mod tests;

const SKY130_DRC_CONFIG_DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tech/sky130/drc_config.yaml"
));

fn tech_config() -> TechConfig {
    TechConfig::from_yaml(SKY130_DRC_CONFIG_DATA).expect("failed to load sky130A tech config")
}

pub fn pdk() -> LayoutResult<Pdk> {
    Pdk::new(tech_config())
}

pub fn pdk_lib(name: impl Into<String>) -> LayoutResult<PdkLib> {
    Ok(PdkLib {
        tech: arcstr::literal!("sky130"),
        pdk: pdk()?,
        lib: Library::new(name, Units::Nano),
    })
}

impl Pdk {
    pub(crate) fn draw_sky130_mos(&self, params: MosParams) -> MosResult<Ref<LayoutTransistors>> {
        params.validate()?;

        let name = params.name();

        let gate_metal = self.li1();
        let sd_metal = self.li1();

        let mut abs = Abstract::new(&name);

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

        let gate_ctp = ContactParams::builder()
            .rows(1)
            .cols(1)
            .dir(CoarseDirection::Horizontal)
            .stack("polyc".into())
            .build()
            .unwrap();
        let gate_ct = self.get_contact(&gate_ctp);
        let gate_bbox = gate_ct.bboxes.get(&self.poly()).unwrap();
        let gate_metal_bbox = gate_ct.bboxes.get(&self.li1()).unwrap();

        let mut gate_pins = Vec::with_capacity(nf as usize);

        let xpoly = x0 - tc.layer("poly").extension("diff");
        let mut ypoly = y0 + diff_edge_to_gate(&tc);
        let wpoly = cx - xpoly + tc.layer("poly").extension("diff");

        // TODO: Need to move gate contacts further away from transistor.
        // There are several relevant design rules, but for now I'll just
        // add a constant offset.
        let poly_fudge_x = 45;
        for i in 0..nf {
            let rect = Rect {
                p0: Point::new(xpoly - poly_fudge_x, ypoly),
                p1: Point::new(xpoly + wpoly, ypoly + params.length()),
            };

            let ofsx = rect.p0.x - gate_bbox.p1.x;
            let ofsy = if i % 2 == 0 {
                rect.p1.y - gate_bbox.p1.y
            } else {
                rect.p0.y - gate_bbox.p0.y
            };

            let ct_ofs = Point::new(ofsx, ofsy);
            let ct_box = translate(gate_metal_bbox, &ct_ofs);
            let mut port = AbstractPort::new(format!("gate_{}", i));
            port.add_shape(gate_metal, Shape::Rect(ct_box));
            abs.add_port(port);
            gate_pins.push(ct_box);

            let inst = Instance {
                inst_name: format!("gate_contact_{}", i),
                cell: Ptr::clone(&gate_ct.cell),
                loc: ct_ofs,
                reflect_vert: false,
                angle: None,
            };

            insts.push(inst);

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

        let mut sd_pins = (0..params.devices.len())
            .map(|_| HashMap::new())
            .collect::<Vec<_>>();

        for i in 0..=nf {
            for (d, (j, x)) in params.devices.iter().zip(diff_xs.iter().enumerate()) {
                if d.skip_sd_metal.contains(&(i as usize)) {
                    continue;
                }
                let ct_stack = match d.mos_type {
                    MosType::Nmos => "ndiffc",
                    MosType::Pmos => "pdiffc",
                };
                let ct = self.get_contact_sized(ct_stack, diff, d.width).unwrap();
                let bbox = ct.bboxes.get(&diff).unwrap();
                let ofsx = (d.width - rect_width(bbox)) / 2;
                let loc = Point::new(x - bbox.p0.x + ofsx, cy - bbox.p0.y);
                let inst = Instance {
                    inst_name: format!("sd_contact_{}_{}", i, j),
                    cell: Ptr::clone(&ct.cell),
                    loc,
                    reflect_vert: false,
                    angle: None,
                };
                insts.push(inst);

                let sd_rect = translate(ct.bboxes.get(&self.li1()).unwrap(), &loc);
                let mut port = AbstractPort::new(format!("sd_{}_{}", j, i));
                port.add_shape(sd_metal, Shape::Rect(sd_rect));
                abs.add_port(port);
                sd_pins[j].insert(i as Uint, Some(sd_rect));
            }
            cy += params.length();
            cy += finger_space(&tc);
        }

        let layout = Layout {
            name: name.clone(),
            insts,
            annotations: vec![],
            elems,
        };

        let cell = Cell {
            name,
            abs: Some(abs),
            layout: Some(layout),
        };

        let transistors = LayoutTransistors {
            cell: Ptr::new(cell),
            sd_metal,
            gate_metal,
            sd_pins,
            gate_pins,
        };

        Ok(Arc::new(transistors))
    }

    pub(crate) fn draw_contact(&self, params: &ContactParams) -> Ref<Contact> {
        let rows = params.rows;
        let cols = params.cols;

        assert!(rows > 0);
        assert!(cols > 0);
        let tc = self.config.read().unwrap();
        let layers = self.layers.read().unwrap();
        let stack_name = params.stack.clone();
        let stack = tc.stack(&stack_name);
        assert_eq!(stack.layers.len(), 3);

        let ctlay_name = &stack.layers[1];
        let ctlay = layers.keyname(&stack.layers[1]).unwrap();

        let mut elems = Vec::new();

        let x0 = 0;
        let y0 = 0;

        let ctw = tc.layer(ctlay_name).width;
        let cts = tc.layer(ctlay_name).space;
        let ctbw = ctw * cols + cts * (cols - 1);
        let ctbh = ctw * rows + cts * (rows - 1);

        let ct_bbox = Rect {
            p0: Point::new(x0, y0),
            p1: Point::new(x0 + ctbw, y0 + ctbh),
        };

        let net_name = "x".to_string();

        for i in 0..rows {
            for j in 0..cols {
                let left = x0 + j * (ctw + cts);
                let bot = y0 + i * (ctw + cts);
                let ct_box = Rect {
                    p0: Point::new(left, bot),
                    p1: Point::new(left + ctw, bot + ctw),
                };

                elems.push(Element {
                    net: None,
                    layer: ctlay,
                    purpose: LayerPurpose::Drawing,
                    inner: Shape::Rect(ct_box),
                });
            }
        }

        let mut bboxes = Vec::with_capacity(2);
        let mut bbox_map = HashMap::with_capacity(3);
        bbox_map.insert(ctlay, ct_bbox.clone());

        let mut aport = AbstractPort {
            net: net_name.clone(),
            shapes: HashMap::new(),
        };

        for lay_name in [&stack.layers[0], &stack.layers[2]] {
            let lay = layers.keyname(lay_name).unwrap();
            let mut laybox = ct_bbox.clone();
            expand_box(&mut laybox, tc.layer(ctlay_name).enclosure(lay_name));
            expand_box_min_width(&mut laybox, tc.layer(lay_name).width, tc.grid);
            let ose = tc.layer(ctlay_name).one_side_enclosure(lay_name);

            match params.dir {
                CoarseDirection::Vertical => {
                    laybox.p0.y = std::cmp::min(laybox.p0.y, ct_bbox.p0.y - ose);
                    laybox.p1.y = std::cmp::max(laybox.p0.y, ct_bbox.p1.y + ose);
                }
                CoarseDirection::Horizontal => {
                    laybox.p0.x = std::cmp::min(laybox.p0.x, ct_bbox.p0.x - ose);
                    laybox.p1.x = std::cmp::max(laybox.p0.x, ct_bbox.p1.x + ose);
                }
            }

            let shape = Shape::Rect(laybox.clone());
            aport.shapes.insert(lay, vec![shape.clone()]);

            bboxes.push(shape.bbox());
            bbox_map.insert(lay, laybox);

            elems.push(Element {
                net: None,
                layer: lay,
                purpose: LayerPurpose::Drawing,
                inner: shape,
            });
        }

        if params.stack == "ndiffc" {
            let mut nsdm_box = rect_from_bbox(&bboxes[1]);
            expand_box(&mut nsdm_box, tc.layer("diff").enclosure("nsdm"));
            elems.push(Element {
                net: None,
                layer: layers.keyname("nsdm").unwrap(),
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(nsdm_box),
            });
        } else if params.stack == "pdiffc" {
            let diff_rect = rect_from_bbox(&bboxes[1]);
            let mut psdm_box = diff_rect.clone();
            expand_box(&mut psdm_box, tc.layer("diff").enclosure("psdm"));
            elems.push(Element {
                net: None,
                layer: layers.keyname("psdm").unwrap(),
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(psdm_box),
            });
            let mut well_box = diff_rect;
            expand_box(&mut well_box, tc.layer("diff").enclosure("nwell"));

            elems.push(Element {
                net: None,
                layer: layers.keyname("nwell").unwrap(),
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(well_box),
            });
        } else if params.stack == "polyc" {
            let mut npc_box = ct_bbox;
            expand_box(&mut npc_box, tc.layer("licon").enclosure("npc"));
            elems.push(Element {
                net: None,
                layer: layers.keyname("nsdm").unwrap(),
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(npc_box),
            });
        }

        let bbox = bboxes[0].union(&bboxes[1]);
        let outline = Rect {
            p0: bbox.p0,
            p1: bbox.p1,
        };

        let name = format!("{}", params);

        let layout = Layout {
            name: name.clone(),
            insts: vec![],
            annotations: vec![],
            elems,
        };

        let abs = Abstract {
            name: name.clone(),
            outline: Some(Element {
                net: Some(net_name),
                layer: layers.keyname(&stack.layers[0]).unwrap(),
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(outline),
            }),
            blockages: HashMap::new(),
            ports: vec![aport],
        };

        let cell = Cell {
            name,
            abs: Some(abs),
            layout: Some(layout),
        };

        let cell = Ptr::new(cell);

        std::sync::Arc::new(Contact {
            cell,
            rows: params.rows,
            cols: params.cols,
            bboxes: bbox_map,
        })
    }

    pub fn metal_name(&self, i: LayerIdx) -> &'static str {
        match i {
            0 => "li",
            1 => "m1",
            2 => "m2",
            3 => "m3",
            4 => "m4",
            5 => "m5",
            _ => panic!("sky130 has no metal layer numbered {}", i),
        }
    }

    pub fn metal(&self, i: LayerIdx) -> LayerKey {
        self.get_layerkey(self.metal_name(i)).unwrap()
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
