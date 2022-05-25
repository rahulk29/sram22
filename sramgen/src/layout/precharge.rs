use layout21::{
    raw::{Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::{
    geometry::CoarseDirection,
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use crate::Result;

fn draw_precharge(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "precharge".to_string();

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
            mos_type: MosType::Pmos,
            width: 600,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_000,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_000,
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
        angle: Some(90f64),
        reflect_vert: false,
    });

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
    fn test_sky130_precharge() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_precharge")?;
        draw_precharge(&mut lib)?;

        lib.save_gds()?;

        Ok(())
    }
}
