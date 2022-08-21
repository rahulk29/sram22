use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, BoundBoxTrait, TransformTrait};
use layout21::{
    raw::{Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::contact::ContactParams;
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
    port.set_net("br0");
    let port = port.transform(&xform);
    abs.add_port(port);

    let mut port = ptx.sd_port(0, 1).unwrap();
    port.set_net("bl0");
    let port = port.transform(&xform);
    abs.add_port(port);

    let mut port = ptx.sd_port(1, 0).unwrap();
    port.set_net("br1");
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

    let mut port = ptx.sd_port(2, 1).unwrap();
    port.set_net("bl1");
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

pub fn draw_tap_cell(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let bot = lib.pdk.get_contact(
        &ContactParams::builder()
            .stack("ntap".to_string())
            .rows(12)
            .cols(1)
            .dir(Dir::Vert)
            .build()
            .unwrap(),
    );
    let top = lib.pdk.get_contact(
        &ContactParams::builder()
            .stack("viali".to_string())
            .rows(11)
            .cols(1)
            .dir(Dir::Vert)
            .build()
            .unwrap(),
    );

    let bot = Instance::new("bot", bot.cell.clone());
    let mut top = Instance::new("top", top.cell.clone());
    top.align_centers_gridded(bot.bbox(), lib.pdk.grid());

    let mut p0 = bot.port("x");
    let p1 = top.port("x");

    p0.merge(p1);

    let name = "pc_tap_cell";

    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);
    abs.add_port(p0);
    layout.add_inst(bot);
    layout.add_inst(top);

    Ok(Ptr::new(Cell {
        layout: Some(layout),
        abs: Some(abs),
        name: name.into(),
    }))
}

pub fn draw_precharge_array(lib: &mut PdkLib, width: usize) -> Result<Ptr<Cell>> {
    assert!(width >= 2);
    let pc = draw_precharge(lib)?;

    let core = draw_cell_array(
        ArrayCellParams {
            name: "precharge_pc_array".to_string(),
            num: width,
            cell: pc,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let tap = draw_tap_cell(lib)?;

    let taps = draw_cell_array(
        ArrayCellParams {
            name: "precharge_tap_array".to_string(),
            num: width + 1,
            cell: tap,
            spacing: Some(2_500),
            flip: FlipMode::AlternateFlipHorizontal,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let mut layout = Layout::new("precharge_array");
    let mut abs = Abstract::new("precharge_array");
    let core = Instance {
        inst_name: "pc_array".to_string(),
        cell: core.cell.clone(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };
    let mut taps = Instance {
        inst_name: "tap_array".to_string(),
        cell: taps.cell.clone(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };
    taps.align_centers_gridded(core.bbox(), lib.pdk.grid());

    abs.ports.append(&mut core.ports());

    let iter = taps.ports().into_iter().enumerate().map(|(i, mut p)| {
        p.set_net(format!("vdd_{}", i));
        p
    });
    abs.ports.extend(iter);
    abs.ports.append(&mut taps.ports());

    layout.add_inst(core);
    layout.add_inst(taps);

    Ok(Ptr::new(Cell {
        name: "precharge_array".to_string(),
        layout: Some(layout),
        abs: Some(abs),
    }))
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_precharge() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_precharge")?;
        draw_precharge(&mut lib)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_precharge_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_precharge_array")?;
        draw_precharge_array(&mut lib, 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
