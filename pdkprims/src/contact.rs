use layout21::raw::{Cell, Element, LayerPurpose, Layout, Point, Rect, Shape};
use layout21::utils::Ptr;

use crate::config::Int;
use crate::{config::Uint, Pdk};

impl Pdk {
    pub fn get_contact(&self, stack: impl Into<String>, rows: Uint, cols: Uint) -> Ptr<Cell> {
        assert!(rows > 0);
        assert!(cols > 0);
        let tc = self.config.read().unwrap();
        let layers = self.layers.read().unwrap();
        let stack_name = stack.into();
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

        for lay_name in [&stack.layers[0], &stack.layers[2]] {
            let lay = layers.keyname(lay_name).unwrap();
            let mut laybox = ct_bbox.clone();
            expand_box(&mut laybox, tc.layer(ctlay_name).enclosure(lay_name));
            let ose = tc.layer(ctlay_name).one_side_enclosure(&lay_name);
            laybox.p0.x = std::cmp::min(laybox.p0.x, ct_bbox.p0.x - ose);
            laybox.p1.x = std::cmp::max(laybox.p0.x, ct_bbox.p1.x + ose);

            elems.push(Element {
                net: None,
                layer: lay,
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(laybox),
            });
        }

        let name = format!("{}_{}x{}", stack_name, rows, cols);

        let layout = Layout {
            name: name.clone(),
            insts: vec![],
            annotations: vec![],
            elems,
        };

        let cell = Cell {
            name,
            abs: None,
            layout: Some(layout),
        };

        Ptr::new(cell)
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
