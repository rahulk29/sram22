use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, AbstractPort, BoundBoxTrait, Rect, Shape, Span, TransformTrait};
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
use super::common::{draw_two_level_contact, TwoLevelContactParams};
use crate::layout::route::{ContactBounds, Router, VertDir};
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
    let params = TwoLevelContactParams::builder()
        .name("pc_tap_cell")
        .bot_stack("ntap")
        .top_stack("viali")
        .bot_rows(12)
        .top_rows(11)
        .build()?;
    let contact = draw_two_level_contact(lib, params)?;
    Ok(contact)
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
        cell: core.cell,
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };
    let mut taps = Instance {
        inst_name: "tap_array".to_string(),
        cell: taps.cell,
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

    let m0 = lib.pdk.metal(0);
    let m2 = lib.pdk.metal(2);

    let mut router = Router::new("precharge_array_route", lib.pdk.clone());
    router.cfg().line(2);

    let pc_b_0 = core.port("pc_b_0").largest_rect(m0).unwrap();

    let span = Span::new(
        pc_b_0.left(),
        core.port(format!("pc_b_{}", width - 1))
            .largest_rect(m0)
            .unwrap()
            .right(),
    );
    let top = pc_b_0.bottom();
    let rect = Rect::span_builder()
        .with(Dir::Horiz, span)
        .with(Dir::Vert, Span::new(top - 3 * router.cfg().line(2), top))
        .build();

    router.trace(rect, 2);

    let mut port = AbstractPort::new("pc_b");
    port.add_shape(m2, Shape::Rect(rect));
    abs.add_port(port);

    for i in 0..width {
        let src = core.port(format!("pc_b_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace.place_cursor(Dir::Vert, false).vert_to(rect.bottom());

        let intersect = trace.rect().intersection(&rect.bbox()).into_rect();
        trace.contact_up(rect).increment_layer().contact_on(
            intersect,
            VertDir::Above,
            ContactBounds::FillDir {
                dir: Dir::Vert,
                size: rect.height(),
                layer: lib.pdk.metal(1),
            },
        );
    }

    layout.add_inst(core);
    layout.add_inst(taps);
    layout.add_inst(router.finish());

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
