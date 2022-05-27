use crate::layout::Result;
use layout21::{
    raw::{Cell, Element, Instance, LayerKey, LayerPurpose, Layout, Point, Rect, Shape},
    utils::Ptr,
};
use pdkprims::{
    geometry::CoarseDirection,
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use super::draw_rect;

fn draw_nand2(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "nand2_dec".to_string();

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

    let ndrain = ptx.sd_pins[0][&2].clone().unwrap();
    let pdrain1 = ptx.sd_pins[1][&2].clone().unwrap();
    let pdrain0 = ptx.sd_pins[1][&0].clone().unwrap();

    let xlim = pdrain0.p0.x - tc.layer("li").space;

    let cx = (ndrain.p1.x + pdrain1.p0.x) / 2;

    let (mut xmin, mut xmax) = lib.pdk.gridded_center_span(cx, tc.layer("li").width);

    if xmax > xlim {
        let xshift = xmax - xlim;
        xmin -= xshift;
        xmax -= xshift;
    }

    layout.elems.push(draw_rect(
        Rect {
            p0: Point::new(ndrain.p0.x, ndrain.p0.y),
            p1: Point::new(pdrain1.p1.x, pdrain1.p1.y),
        },
        ptx.sd_metal,
    ));

    layout.elems.push(draw_rect(
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(xmax, pdrain1.p1.y),
        },
        ptx.sd_metal,
    ));

    layout.elems.push(draw_rect(
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(pdrain0.p1.x, pdrain0.p1.y),
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

fn draw_inv_dec(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
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

    let ndrain = ptx.sd_pins[0][&2].clone().unwrap();
    let pdrain1 = ptx.sd_pins[1][&2].clone().unwrap();
    let pdrain0 = ptx.sd_pins[1][&0].clone().unwrap();

    let xlim = pdrain0.p0.x - tc.layer("li").space;

    let cx = (ndrain.p1.x + pdrain1.p0.x) / 2;

    let (mut xmin, mut xmax) = lib.pdk.gridded_center_span(cx, tc.layer("li").width);

    if xmax > xlim {
        let xshift = xmax - xlim;
        xmin -= xshift;
        xmax -= xshift;
    }

    layout.elems.push(draw_rect(
        Rect {
            p0: Point::new(ndrain.p0.x, ndrain.p0.y),
            p1: Point::new(pdrain1.p1.x, pdrain1.p1.y),
        },
        ptx.sd_metal,
    ));

    layout.elems.push(draw_rect(
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(xmax, pdrain1.p1.y),
        },
        ptx.sd_metal,
    ));

    layout.elems.push(draw_rect(
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(pdrain0.p1.x, pdrain0.p1.y),
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
}
