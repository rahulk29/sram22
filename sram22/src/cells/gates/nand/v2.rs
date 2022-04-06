use std::path::{Path, PathBuf};

use layout21::{
    raw::{Cell, Element, Instance, LayerPurpose, Layout, Point, Rect, Shape},
    utils::Ptr,
};
use pdkprims::{
    geometry::CoarseDirection,
    mos::{Intent, MosDevice, MosParams, MosType},
    Pdk,
};

pub fn draw_nand2(pdk: &Pdk) -> Result<Ptr<Cell>, Box<dyn std::error::Error>> {
    let name = "nand2_dec";

    let mut layout = Layout {
        name: name.to_string(),
        insts: Vec::with_capacity(1),
        elems: Vec::with_capacity(3),
        annotations: vec![],
    };

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(CoarseDirection::Horizontal)
        .add_device(
            MosDevice::builder()
                .mos_type(MosType::Nmos)
                .width(1_000)
                .length(150)
                .fingers(2)
                .skip_sd_metal(vec![1])
                .build()
                .unwrap(),
        )
        .add_device(
            MosDevice::builder()
                .mos_type(MosType::Pmos)
                .width(1_400)
                .length(150)
                .fingers(2)
                .build()
                .unwrap(),
        );
    let gate = pdk.draw_mos(params)?;

    layout.insts.push(Instance {
        inst_name: "transistors".into(),
        cell: Ptr::clone(&gate.cell),
        loc: Point::new(0, 0),
        angle: None,
        reflect_vert: false,
    });

    let yn = &gate.sd_pins[0][&2].as_ref().unwrap();
    let vss = &gate.sd_pins[0][&0].as_ref().unwrap();
    let ypt = &gate.sd_pins[1][&2].as_ref().unwrap();
    let ypb = &gate.sd_pins[1][&0].as_ref().unwrap();

    let config = pdk.config();
    let config = config.read().unwrap();
    let min_x = vss.p1.x + config.layer("li").space;

    let top_met = Rect {
        p0: Point::new(yn.p0.x, yn.p0.y),
        p1: Point::new(ypt.p1.x, ypt.p1.y),
    };

    let vert_met = Rect {
        p0: Point::new(min_x, ypb.p0.y),
        p1: Point::new(min_x + config.layer("li").width, ypt.p1.y),
    };

    let bot_met = Rect {
        p0: Point::new(min_x, ypb.p0.y),
        p1: Point::new(ypb.p1.x, ypb.p1.y),
    };

    for rect in [top_met, vert_met, bot_met] {
        layout.elems.push(Element {
            net: Some("y".into()),
            layer: pdk.get_layerkey("li").unwrap(),
            purpose: LayerPurpose::Drawing,
            inner: Shape::Rect(rect),
        });
    }

    let cell = layout21::raw::Cell {
        name: name.to_string(),
        abs: None,
        layout: Some(layout),
    };

    Ok(Ptr::new(cell))
}

#[cfg(test)]
mod tests {
    use super::{draw_nand2, output};

    #[test]
    fn test_draw_nand2() -> Result<(), Box<dyn std::error::Error>> {
        let pdk = pdkprims::tech::sky130::pdk()?;
        let cell = draw_nand2(&pdk)?;
        pdk.cell_to_gds(cell, output("test_draw_nand2.gds"))?;
        Ok(())
    }
}

pub fn output(name: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../_build/")
        .join(name)
}
