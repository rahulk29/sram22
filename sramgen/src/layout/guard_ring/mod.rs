use layout21::raw::align::AlignRect;
use layout21::raw::{BoundBoxTrait, Cell, Instance, Point, Rect, Span};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::common::{draw_two_level_contact, rect_cutout, TwoLevelContactParams};
use super::route::Router;

pub mod ring;

pub struct GuardRingParams {
    pub enclosure: Rect,
    pub prefix: String,
}

pub const WIDTH_MULTIPLIER: isize = 8;
pub const DNW_ENCLOSURE: isize = 400;
pub const NWELL_HOLE_ENCLOSURE: isize = 1_030;

pub fn draw_guard_ring(lib: &mut PdkLib, params: GuardRingParams) -> crate::Result<Ptr<Cell>> {
    let GuardRingParams { enclosure, prefix } = params;
    let h_metal = 2;
    let v_metal = 1;

    let nwell_width = DNW_ENCLOSURE + NWELL_HOLE_ENCLOSURE;

    let mut router = Router::new(format!("{}_route", &prefix), lib.pdk.clone());
    let cfg = router.cfg();

    let t_span = Span::new(
        enclosure.top(),
        enclosure.top() + WIDTH_MULTIPLIER * cfg.line(h_metal),
    );
    let b_span = Span::new(
        enclosure.bottom() - WIDTH_MULTIPLIER * cfg.line(h_metal),
        enclosure.bottom(),
    );

    let l_span = Span::new(
        enclosure.left() - WIDTH_MULTIPLIER * cfg.line(v_metal),
        enclosure.left(),
    );
    let r_span = Span::new(
        enclosure.right(),
        enclosure.right() + WIDTH_MULTIPLIER * cfg.line(v_metal),
    );

    let v_span = Span::new(b_span.start(), t_span.stop());
    let h_span = Span::new(l_span.start(), r_span.stop());

    let left = Rect::from_spans(l_span, v_span);
    let right = Rect::from_spans(r_span, v_span);
    let bot = Rect::from_spans(h_span, b_span);
    let top = Rect::from_spans(h_span, t_span);

    let left_trace = router.trace(left, v_metal);
    let right_trace = router.trace(right, v_metal);
    let mut bot_trace = router.trace(bot, h_metal);
    let mut top_trace = router.trace(top, h_metal);

    top_trace
        .contact_down(left_trace.rect())
        .contact_down(right_trace.rect());
    bot_trace
        .contact_down(left_trace.rect())
        .contact_down(right_trace.rect());

    let ctp = TwoLevelContactParams::builder()
        .name(format!("{}_contact", &prefix))
        .bot_stack("ntap")
        .top_stack("viali")
        .build()?;

    let contact = draw_two_level_contact(lib, ctp)?;
    let (width, height) = {
        let ct = contact.read().unwrap();
        let bbox = ct.layout().bbox();
        (bbox.width(), bbox.height())
    };

    let mut cell = Cell::empty(&prefix);

    let area = params.enclosure.expand(400);

    let m1 = cfg.layerkey(1);

    let mut x = area.left() + 2 * width;
    while x < area.right() - 2 * width {
        for target in [top, bot] {
            let mut inst = Instance::new("contact", contact.clone());
            inst.loc = Point::new(x, 0);
            inst.align_centers_vertically_gridded(target.bbox(), cfg.grid());
            let src = inst.port("x").largest_rect(m1).unwrap();
            let mut trace = router.trace(src, 1);
            trace.contact_up(target);
            cell.layout_mut().add_inst(inst);
        }
        x += 3 * width;
    }

    let mut y = area.bottom() + 2 * height;
    while y < area.top() - 2 * height {
        let mut inst = Instance::new("contact", contact.clone());
        inst.loc = Point::new(area.left(), y);
        inst.align_centers_horizontally_gridded(left.bbox(), cfg.grid());
        cell.layout_mut().add_inst(inst);

        let mut inst = Instance::new("contact", contact.clone());
        inst.loc = Point::new(area.right(), y);
        inst.align_centers_horizontally_gridded(right.bbox(), cfg.grid());
        y += 3 * height;
        cell.layout_mut().add_inst(inst);
    }

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();
    let dnw = lib.pdk.get_layerkey("dnwell").unwrap();
    let dnw_boundary = enclosure.expand(NWELL_HOLE_ENCLOSURE);
    let nwell_boundary = enclosure.expand(nwell_width);

    for rect in rect_cutout(nwell_boundary, enclosure) {
        cell.layout_mut().draw_rect(nwell, rect);
    }
    cell.layout_mut().draw_rect(dnw, dnw_boundary);

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;
    use crate::Result;

    use super::*;

    #[test]
    fn square_200um() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_guard_ring_square_200um")?;
        draw_guard_ring(
            &mut lib,
            GuardRingParams {
                enclosure: Rect::new(Point::zero(), Point::new(200_000, 200_000)),
                prefix: "test_guard_ring_square_200um".to_string(),
            },
        )?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
