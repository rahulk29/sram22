use std::collections::HashMap;
use std::fmt::Display;

use layout21::raw::{
    Abstract, AbstractPort, BoundBox, BoundBoxTrait, Cell, Element, LayerKey, LayerPurpose, Layout,
    Point, Rect, Shape,
};
use layout21::utils::Ptr;
use serde::{Deserialize, Serialize};

use crate::config::Int;
use crate::geometry::CoarseDirection;
use crate::{config::Uint, Pdk};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, derive_builder::Builder)]
pub struct ContactParams {
    pub stack: String,
    pub rows: Uint,
    pub cols: Uint,
    /// The "relaxed" direction, ie. the direction in which there is more margin (for overhangs,
    /// for instance).
    ///
    /// If the contact generator needs more space, it will try to expand in
    /// this direction first.
    pub dir: CoarseDirection,
}

#[derive(Debug, Clone, Eq, PartialEq, derive_builder::Builder)]
pub struct Contact {
    pub cell: Ptr<Cell>,
    pub rows: Uint,
    pub cols: Uint,
    pub bboxes: HashMap<LayerKey, Rect>,
}

impl Display for ContactParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}x{}{}",
            &self.stack,
            self.rows,
            self.cols,
            self.dir.short_form()
        )
    }
}

impl ContactParams {
    pub fn builder() -> ContactParamsBuilder {
        ContactParamsBuilder::default()
    }
}

impl Pdk {
    pub fn get_contact(&self, params: &ContactParams) -> Contact {
        let mut map = self.contacts.write().unwrap();
        if let Some(c) = map.get(&params) {
            c.clone()
        } else {
            let c = self.draw_contact(&params);
            map.insert(params.to_owned(), c.clone());
            c
        }
    }

    pub fn get_contact_sized(
        &self,
        stack: impl Into<String>,
        layer: LayerKey,
        width: Int,
    ) -> Option<Contact> {
        let mut low = 1;
        let mut high = 100;
        let mut result = None;

        let stack = stack.into();

        while high > low {
            let mid = (high + low) / 2;
            let params = ContactParams::builder()
                .rows(1)
                .cols(mid)
                .stack(stack.clone())
                .dir(CoarseDirection::Horizontal)
                .build()
                .unwrap();
            let ct = self.get_contact(&params);
            let bbox = ct.bboxes.get(&layer).unwrap();

            if bbox.p1.x - bbox.p0.x < width {
                result = Some(ct);
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        result
    }

    fn draw_contact(&self, params: &ContactParams) -> Contact {
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

        let ctw = tc.layer(&ctlay_name).width;
        let cts = tc.layer(&ctlay_name).space;
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
            let ose = tc.layer(ctlay_name).one_side_enclosure(&lay_name);

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
                net: Some(net_name.clone()),
                layer: lay,
                purpose: LayerPurpose::Drawing,
                inner: shape,
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
            outline: Element {
                net: Some(net_name.clone()),
                layer: layers.keyname(&stack.layers[0]).unwrap(),
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(outline),
            },
            blockages: HashMap::new(),
            ports: vec![aport],
        };

        let cell = Cell {
            name,
            abs: Some(abs),
            layout: Some(layout),
        };

        let cell = Ptr::new(cell);

        Contact {
            cell,
            rows: params.rows,
            cols: params.cols,
            bboxes: bbox_map,
        }
    }
}

fn expand_box(b: &mut Rect, dist: Int) {
    assert!(b.p0.x <= b.p1.x);
    assert!(b.p0.y <= b.p1.y);

    b.p0.x -= dist;
    b.p1.x += dist;
    b.p0.y -= dist;
    b.p1.y += dist;
}

fn rect_from_bbox(bbox: &BoundBox) -> Rect {
    Rect {
        p0: bbox.p0.clone(),
        p1: bbox.p1.clone(),
    }
}
