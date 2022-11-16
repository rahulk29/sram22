use crate::schematic::wmask_control::WriteMaskControlParams;
use crate::Result;

use layout21::raw::{BoundBoxTrait, Cell, Dir, Instance, Rect, Span};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::decoder::draw_and2_array;
use super::route::Router;

pub fn draw_write_mask_control(
    lib: &mut PdkLib,
    params: WriteMaskControlParams,
) -> Result<Ptr<Cell>> {
    let WriteMaskControlParams {
        name,
        width,
        and_params,
    } = params;
    let width = width as usize;
    let mut cell = Cell::empty(&name);
    let nand = and_params.nand;
    let inv = and_params.inv;

    let and2_array = draw_and2_array(lib, &format!("{}_and2_array", &name), width, nand, inv)?;
    let and2_array = Instance::new("and2_array", and2_array);

    let mut router = Router::new(format!("{}_route", name), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    let bbox = and2_array.bbox().into_rect();

    for port in [
        "vnb0", "vss0", "vdd0", "vpb0", "vnb1", "vss1", "vdd1", "vpb1",
    ] {
        let rect = and2_array.port(port).largest_rect(m1).unwrap();
        let rect = Rect::from_spans(rect.hspan(), bbox.vspan());
        cell.layout_mut().draw_rect(m1, rect);
        cell.add_pin(port, m1, rect);
    }

    let wr_en_rect = Rect::from_spans(
        Span::new(
            bbox.left() - cfg.line(1) - cfg.space(1) * 3 / 2,
            bbox.left() - cfg.space(1) * 3 / 2,
        ),
        bbox.vspan(),
    );
    cell.layout_mut().draw_rect(m1, wr_en_rect);

    for i in 0..width {
        let src = and2_array.port(format!("b_{i}")).largest_rect(m0).unwrap();
        let mut trace = router.trace(src, 0);
        trace
            .place_cursor(Dir::Horiz, false)
            .horiz_to(wr_en_rect.left())
            .contact_up(wr_en_rect);

        cell.add_pin_from_port(
            and2_array.port(format!("a_{i}")).named(format!("sel_{i}")),
            m0,
        );
        cell.add_pin_from_port(
            and2_array
                .port(format!("y_{i}"))
                .named(format!("write_driver_en_{i}")),
            m0,
        );
    }

    cell.add_pin("wr_en", m1, wr_en_rect);

    cell.layout_mut().add_inst(and2_array);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
