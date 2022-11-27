use layout21::raw::align::AlignRect;
use layout21::raw::{BoundBoxTrait, Cell, Instance, Point, Rect};
use layout21::utils::Ptr;
use pdkprims::{LayerIdx, PdkLib};

use self::ring::{Ring, RingParams};

use super::common::{draw_two_level_contact, rect_cutout, TwoLevelContactParams};
use super::route::Router;

pub mod ring;

pub struct GuardRingParams {
    pub enclosure: Rect,
    pub prefix: String,
}

pub struct GuardRing {
    pub cell: Ptr<Cell>,
    pub vdd_ring: Ring,
    pub vss_ring: Ring,
    pub h_metal: LayerIdx,
    pub v_metal: LayerIdx,
}

pub const WIDTH_MULTIPLIER: isize = 8;
pub const DNW_ENCLOSURE: isize = 440;
pub const NWELL_HOLE_ENCLOSURE: isize = 1_080;

pub fn draw_guard_ring(lib: &mut PdkLib, params: GuardRingParams) -> crate::Result<GuardRing> {
    let GuardRingParams { enclosure, prefix } = params;
    let h_metal = 2;
    let v_metal = 1;

    let nwell_width = DNW_ENCLOSURE + NWELL_HOLE_ENCLOSURE;

    let mut router = Router::new(format!("{}_route", &prefix), lib.pdk.clone());
    let cfg = router.cfg();

    let vss_ring = RingParams::builder()
        .enclosure(enclosure)
        .h_width(WIDTH_MULTIPLIER * cfg.line(h_metal))
        .v_width(WIDTH_MULTIPLIER * cfg.line(v_metal))
        .build()?
        .draw();
    let vdd_ring = RingParams::builder()
        .enclosure(vss_ring.outer_enclosure().expand(3 * cfg.space(2)))
        .h_width(WIDTH_MULTIPLIER * cfg.line(h_metal))
        .v_width(WIDTH_MULTIPLIER * cfg.line(v_metal))
        .build()?
        .draw();

    let mut cell = Cell::empty(&prefix);

    for (net, ring) in [("vss", vss_ring), ("vdd", vdd_ring)] {
        let left_trace = router.trace(ring.left(), v_metal);
        let right_trace = router.trace(ring.right(), v_metal);
        let mut bot_trace = router.trace(ring.bottom(), h_metal);
        let mut top_trace = router.trace(ring.top(), h_metal);

        top_trace
            .contact_down(left_trace.rect())
            .contact_down(right_trace.rect());
        bot_trace
            .contact_down(left_trace.rect())
            .contact_down(right_trace.rect());

        let ctp = TwoLevelContactParams::builder()
            .name(format!("{}_{}_contact", &prefix, &net))
            .bot_stack(if net == "vss" { "ptap" } else { "ntap" })
            .top_stack("viali")
            .build()?;

        let contact = draw_two_level_contact(lib, ctp)?;
        let (width, height) = {
            let ct = contact.read().unwrap();
            let bbox = ct.layout().bbox();
            (bbox.width(), bbox.height())
        };

        let area = ring.outer_enclosure();

        let m1 = cfg.layerkey(1);

        let mut x = area.left() + 2 * ring.left().width();
        while x < area.right() - 2 * ring.left().width() {
            for target in [ring.top(), ring.bottom()] {
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
            inst.align_centers_horizontally_gridded(ring.left().bbox(), cfg.grid());
            cell.layout_mut().add_inst(inst);

            let mut inst = Instance::new("contact", contact.clone());
            inst.loc = Point::new(area.right(), y);
            inst.align_centers_horizontally_gridded(ring.right().bbox(), cfg.grid());
            y += 3 * height;
            cell.layout_mut().add_inst(inst);
        }
    }

    let nwell = lib.pdk.get_layerkey("nwell").unwrap();
    let dnw = lib.pdk.get_layerkey("dnwell").unwrap();
    let dnw_boundary = vdd_ring.inner_enclosure().expand(NWELL_HOLE_ENCLOSURE);
    let nwell_boundary = vdd_ring.inner_enclosure().expand(nwell_width);

    for rect in rect_cutout(nwell_boundary, vdd_ring.inner_enclosure()) {
        cell.layout_mut().draw_rect(nwell, rect);
    }
    cell.layout_mut().draw_rect(dnw, dnw_boundary);

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(GuardRing {
        cell: ptr,
        vdd_ring,
        vss_ring,
        h_metal,
        v_metal,
    })
}
