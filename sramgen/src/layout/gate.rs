use std::collections::HashMap;

use crate::layout::Result;
use layout21::{
    raw::{Abstract, AbstractPort, Cell, Instance, Layout, Point, Rect, Shape},
    utils::Ptr,
};
use pdkprims::{
    geometry::CoarseDirection,
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use super::draw_rect;

pub fn draw_nand2(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "nand2_dec".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let mut abs = Abstract::new(&name);

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(CoarseDirection::Horizontal)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: 1_000,
            length: 150,
            fingers: 2,
            intent: Intent::Svt,
            skip_sd_metal: vec![1],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_400,
            length: 150,
            fingers: 2,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    layout.insts.push(Instance {
        inst_name: "mos".to_string(),
        cell: ptx.cell.clone(),
        loc: Point::new(0, 0),
        angle: None,
        reflect_vert: false,
    });

    let tc = lib.pdk.config.read().unwrap();

    let ndrain = ptx.sd_pin(0, 2).unwrap();
    let pdrain1 = ptx.sd_pin(1, 2).unwrap();
    let pdrain0 = ptx.sd_pin(1, 0).unwrap();

    let xlim = pdrain0.p0.x - tc.layer("li").space;

    let cx = (ndrain.p1.x + pdrain1.p0.x) / 2;

    let (mut xmin, mut xmax) = lib.pdk.gridded_center_span(cx, tc.layer("li").width);

    if xmax > xlim {
        let xshift = xmax - xlim;
        xmin -= xshift;
        xmax -= xshift;
    }

    let mut port_vss = ptx.sd_port(0, 0).unwrap();
    port_vss.set_net("VSS");

    let mut port_vdd = AbstractPort::new("VDD");
    port_vdd.set_net("VDD");

    let mut port_a = ptx.gate_port(0).unwrap();
    port_a.set_net("A");

    let mut port_b = ptx.gate_port(1).unwrap();
    port_b.set_net("B");

    let mut port_y = AbstractPort::new("Y");

    let rects = [
        Rect {
            p0: Point::new(ndrain.p0.x, ndrain.p0.y),
            p1: Point::new(pdrain1.p1.x, pdrain1.p1.y),
        },
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(xmax, pdrain1.p1.y),
        },
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(pdrain0.p1.x, pdrain0.p1.y),
        },
    ];

    for r in rects {
        layout.elems.push(draw_rect(r, ptx.sd_metal));
        port_y.add_shape(ptx.sd_metal, Shape::Rect(r));
    }

    abs.add_port(port_y);

    let cell = Cell {
        name,
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_inv_dec(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "inv_dec".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(CoarseDirection::Horizontal)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: 2_000,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 2_800,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    layout.insts.push(Instance {
        inst_name: "mos".to_string(),
        cell: ptx.cell.clone(),
        loc: Point::new(0, 0),
        angle: None,
        reflect_vert: false,
    });

    let dout_n = ptx.sd_pins[0][&1].clone().unwrap();
    let dout_p = ptx.sd_pins[1][&1].clone().unwrap();

    layout.elems.push(draw_rect(
        Rect {
            p0: Point::new(dout_n.p0.x, dout_n.p0.y),
            p1: Point::new(dout_p.p1.x, dout_p.p1.y),
        },
        ptx.sd_metal,
    ));

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;

    #[test]
    fn test_sky130_nand2() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_nand2")?;
        draw_nand2(&mut lib)?;

        lib.save_gds()?;

        Ok(())
    }

    #[test]
    fn test_sky130_inv_dec() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_inv_dec")?;
        draw_inv_dec(&mut lib)?;

        lib.save_gds()?;

        Ok(())
    }
}
