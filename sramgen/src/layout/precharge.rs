use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, TransformTrait};
use layout21::{
    raw::{Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::{
    mos::{Intent, MosDevice, MosParams, MosType},
    PdkLib,
};

use super::array::*;
use crate::Result;

fn draw_precharge(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "precharge".to_string();

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
        .direction(Dir::Horiz)
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
            width: 1_200,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_200,
            length: 150,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    let inst = Instance {
        inst_name: "mos".to_string(),
        cell: ptx.cell.clone(),
        loc: Point::new(0, 0),
        angle: Some(90f64),
        reflect_vert: false,
    };
    let xform = inst.transform();

    layout.insts.push(inst);

    let mut port = ptx.gate_port(0).unwrap();
    port.set_net("pc_b");
    let port = port.transform(&xform);
    abs.add_port(port);

    let mut port = ptx.sd_port(0, 0).unwrap();
    port.set_net("bl0");
    let port = port.transform(&xform);
    abs.add_port(port);

    let mut port = ptx.sd_port(0, 1).unwrap();
    port.set_net("br0");
    let port = port.transform(&xform);
    abs.add_port(port);

    let mut port = ptx.sd_port(1, 0).unwrap();
    port.set_net("bl1");
    let port = port.transform(&xform);
    abs.add_port(port);

    let mut port = ptx.sd_port(1, 1).unwrap();
    port.set_net("vdd0");
    let port = port.transform(&xform);
    abs.add_port(port);

    let mut port = ptx.sd_port(2, 0).unwrap();
    port.set_net("vdd1");
    let port = port.transform(&xform);
    abs.add_port(port);

    let cell = Cell {
        name,
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_precharge_array(lib: &mut PdkLib, width: usize) -> Result<ArrayedCell> {
    let pc = draw_precharge(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "precharge_array".to_string(),
            num: width,
            cell: pc,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )
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

    #[test]
    fn test_sky130_precharge_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_precharge_array")?;
        draw_precharge_array(&mut lib, 32)?;

        lib.save_gds()?;

        Ok(())
    }
}
