use layout21::raw::{AbstractPort, BoundBox, BoundBoxTrait, Cell, Instance, Point, Rect, Shape};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use crate::config::inv_chain::{InvChainGridParams, InvChainParams};
use crate::layout::common::{sc_outline, MergeArgs};
use crate::layout::route::Router;
use crate::layout::sram::GateList;
use crate::tech::{sc_inv_gds, sc_tap_gds};
use crate::{bus_bit, Result};

pub fn draw_inv_chain(lib: &mut PdkLib, params: &InvChainParams) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty(&params.name);

    let inv = sc_inv_gds(lib)?;
    let tap = sc_tap_gds(lib)?;

    let tap0 = Instance::new("tap0", tap.clone());
    let tmp = Instance::new("", inv.clone());
    let inv_outline = sc_outline(&lib.pdk, &tmp);
    let tap_outline = sc_outline(&lib.pdk, &tap0);

    let mut router = Router::new(format!("{}_route", params.name), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    let mut x = tap_outline.p1.x;
    let mut prev: Option<Instance> = None;
    for i in 0..params.num {
        let mut inv = Instance::new(format!("inv_{}", i), inv.clone());
        inv.loc.x = x;
        x += inv_outline.width();
        if let Some(prev) = prev {
            let dst = prev.port("y").largest_rect(m0).unwrap();
            let src = inv.port("a").largest_rect(m0).unwrap();

            let mut trace = router.trace(src, 0);
            trace
                .place_cursor(layout21::raw::Dir::Horiz, false)
                .horiz_to(dst.left());
        }
        cell.layout_mut().add_inst(inv.clone());

        if i == 0 {
            let rect = inv.port("a").largest_rect(m0).unwrap();
            cell.add_pin("din", m0, rect);
        } else if i == params.num - 1 {
            let rect = inv.port("y").largest_rect(m0).unwrap();
            cell.add_pin("dout", m0, rect);
        }

        prev = Some(inv);
    }

    let outline = Rect::new(
        Point::new(0, 0),
        Point::new(
            2 * tap_outline.width() + params.num as isize * inv_outline.width(),
            tap_outline.height(),
        ),
    );
    let outline_layer = lib.pdk.get_layerkey("outline").unwrap();
    cell.layout_mut().draw_rect(outline_layer, outline);

    let mut tap1 = Instance::new("tap1", tap);
    tap1.loc.x = x;

    cell.layout_mut().add_inst(tap0);
    cell.layout_mut().add_inst(tap1);

    let rect = MergeArgs::builder()
        .layer(m1)
        .insts(GateList::Cells(&cell.layout().insts))
        .port_name("vgnd")
        .build()?
        .rect();
    cell.add_pin("vgnd", m1, rect);

    let rect = MergeArgs::builder()
        .layer(m1)
        .insts(GateList::Cells(&cell.layout().insts))
        .port_name("vpwr")
        .build()?
        .rect();
    cell.add_pin("vpwr", m1, rect);

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_inv_chain_grid(lib: &mut PdkLib, params: &InvChainGridParams) -> Result<Ptr<Cell>> {
    let &InvChainGridParams { rows, cols, .. } = params;
    let name = &params.name;
    let mut cell = Cell::empty(name);

    let inv = sc_inv_gds(lib)?;
    let tap = sc_tap_gds(lib)?;

    let tap0 = Instance::new("", tap.clone());
    let inv0 = Instance::new("", inv.clone());
    let tap_outline = sc_outline(&lib.pdk, &tap0);
    let inv_outline = sc_outline(&lib.pdk, &inv0);

    assert_eq!(tap_outline.height(), inv_outline.height());

    let mut router = Router::new(format!("{}_route", name), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    let mut m2_rects = Vec::with_capacity(rows);

    let start_x = 0;
    let mut x;
    let mut y = 0;
    let mut prev: Option<Instance> = None;
    let mut row_cells = Vec::with_capacity(cols + 2);
    for j in 0..rows {
        x = start_x;
        row_cells.clear();

        let mut xtap = Instance::new(format!("tap_left_{j}"), tap.clone());
        xtap.loc = Point::new(x, y);
        x += tap_outline.width();
        if j % 2 != 0 {
            xtap.reflect_vert_anchored();
        }
        row_cells.push(xtap.clone());
        cell.layout_mut().add_inst(xtap);

        for i in 0..cols {
            let mut xinv = Instance::new(format!("inv_{j}_{i}"), inv.clone());
            xinv.loc = Point::new(x, y);
            x += inv_outline.width();
            if j % 2 != 0 {
                xinv.reflect_vert_anchored();
            }
            if i == 0 && j == 0 {
                let rect = xinv.port("a").largest_rect(m0).unwrap();
                cell.add_pin("din", m0, rect);
            } else if i == cols - 1 && j == rows - 1 {
                let rect = xinv.port("y").largest_rect(m0).unwrap();
                cell.add_pin("dout", m0, rect);
            }
            // Routing - connect the previous inverter to the next one in the chain
            if let Some(prev) = prev {
                let dst = prev.port("y").largest_rect(m0).unwrap();
                let src = xinv.port("a").largest_rect(m0).unwrap();

                if i != 0 {
                    // Within row routing
                    let mut trace = router.trace(src, 0);
                    trace
                        .place_cursor(layout21::raw::Dir::Horiz, false)
                        .horiz_to(dst.left());
                } else {
                    // Route from previous row
                    let mut trace = router.trace(dst, 0);
                    trace
                        .place_cursor_centered()
                        .up()
                        .left_by(600)
                        .up()
                        .vert_to(src.top() + 500);
                    m2_rects.push(trace.rect());
                    trace
                        .down()
                        .horiz_to_rect(src)
                        .vert_to_rect(src)
                        .contact_down(src);
                }
            }
            cell.layout_mut().add_inst(xinv.clone());
            row_cells.push(xinv.clone());
            prev = Some(xinv);
        }

        let mut xtap = Instance::new(format!("tap_right_{j}"), tap.clone());
        xtap.loc = Point::new(x, y);
        if j % 2 != 0 {
            xtap.reflect_vert_anchored();
        }
        row_cells.push(xtap.clone());
        cell.layout_mut().add_inst(xtap);

        if j % 2 != 0 {
            let rect = MergeArgs::builder()
                .layer(m1)
                .insts(GateList::Cells(&row_cells))
                .port_name("vgnd")
                .build()?
                .rect();
            cell.add_pin(bus_bit("vgnd", j / 2), m1, rect);
        } else {
            let rect = MergeArgs::builder()
                .layer(m1)
                .insts(GateList::Cells(&row_cells))
                .port_name("vpwr")
                .build()?
                .rect();
            cell.add_pin(bus_bit("vpwr", j / 2), m1, rect);
        }

        // Special case: need to handle the final power rail
        if j == rows - 1 {
            let (iport, xport) = if j % 2 != 0 {
                ("vpwr", "vpwr")
            } else {
                ("vgnd", "vgnd")
            };

            let rect = MergeArgs::builder()
                .layer(m1)
                .insts(GateList::Cells(&row_cells))
                .port_name(iport)
                .build()?
                .rect();
            cell.add_pin(bus_bit(xport, j / 2), m1, rect);
        }

        y -= tap_outline.height();
    }

    // Export m2 blockage
    let mut bbox = BoundBox::empty();
    for r in m2_rects {
        bbox = bbox.union(&r.bbox());
    }

    let mut p = AbstractPort::new("m2_block");
    p.add_shape(m2, Shape::Rect(bbox.into_rect()));
    cell.abs_mut().add_port(p);

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
