use anyhow::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Dir, Instance, Layout, Rect, Shape, Span,
};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use crate::gate::{GateParams, Size};

use super::gate::draw_nor2;
use super::route::Router;

pub fn draw_sr_latch(lib: &mut PdkLib, name: &str) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);

    let nor = draw_nor2(
        lib,
        GateParams {
            name: format!("{}_nor2", name),
            size: Size {
                nmos_width: 1_500,
                pmos_width: 3_000,
            },
            length: 150,
        },
    )?;

    let nor1 = Instance::new("nor1", nor.clone());
    let mut nor2 = Instance::new("nor2", nor);
    nor2.reflect_vert = true;

    let nor1_bbox = nor1.bbox();
    nor2.align_beneath(nor1_bbox, 200);
    let nor2_bbox = nor2.bbox();

    let mut router = Router::new(format!("{}_route", name), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    let src = nor2.port("y").largest_rect(m0).unwrap();
    let dst = nor1.port("a").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor(Dir::Horiz, false)
        .vert_to(src.p1.y + cfg.line(0) + cfg.space(0))
        .horiz_to(dst.p0.x)
        .vert_to(dst.p1.y);

    let src = nor1.port("y").largest_rect(m0).unwrap();
    let dst = nor2.port("a").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace
        .place_cursor(Dir::Horiz, false)
        .up()
        .horiz_to(dst.p0.x)
        .vert_to(dst.p0.y)
        .contact_down(dst);

    abs.add_port(nor1.port("b").named("set"));
    abs.add_port(nor2.port("b").named("reset"));
    abs.add_port(nor1.port("y").named("q_b"));
    abs.add_port(nor2.port("y").named("q"));

    let width = 3 * cfg.line(1);
    for port in ["vdd", "vss"] {
        let src = nor1.port(port).largest_rect(m0).unwrap();
        let src2 = nor2.port(port).largest_rect(m0).unwrap();
        let xspan = Span::from_center_span_gridded(src.center().x, width, cfg.grid());
        let span = Span::new(nor2_bbox.p0.y, nor1_bbox.p1.y);
        let rect = Rect::span_builder()
            .with(Dir::Vert, span)
            .with(Dir::Horiz, xspan)
            .build();
        let mut trace = router.trace(rect, 1);
        trace.contact_down(src).contact_down(src2);
        let mut port = AbstractPort::new(port.to_lowercase());
        port.add_shape(m1, Shape::Rect(rect));
        abs.add_port(port);
    }

    layout.add_inst(nor1);
    layout.add_inst(nor2);
    layout.add_inst(router.finish());

    let ptr = Ptr::new(Cell {
        name: name.to_string(),
        layout: Some(layout),
        abs: Some(abs),
    });
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_sr_latch() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_sr_latch")?;
        draw_sr_latch(&mut lib, "test_sky130_sr_latch")?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
