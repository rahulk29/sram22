use std::collections::HashMap;

use layout21::raw::geom::Dir;
use layout21::raw::{
    Abstract, BoundBox, BoundBoxTrait, Cell, Instance, Int, Layout, Point, Rect, Span,
};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use crate::bbox;
use crate::layout::grid::GridCells;
use crate::tech::*;
use serde::{Deserialize, Serialize};

use super::route::Router;
use super::Result;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FlipMode {
    None,
    AlternateFlipVertical,
    AlternateFlipHorizontal,
}

pub struct ArrayCellParams {
    pub name: String,
    pub num: usize,
    pub cell: Ptr<Cell>,
    pub spacing: Option<Int>,
    pub flip: FlipMode,
    pub direction: Dir,
    /// By default, cells 0, 2, 4, ... will be flipped according to the flip mode.
    /// If `flip_toggle` is set, cells 1, 3, 5, ... will be flipped instead.
    pub flip_toggle: bool,
}

pub struct ArrayedCell {
    pub cell: Ptr<Cell>,
}

pub fn draw_cell_array(params: ArrayCellParams, lib: &mut PdkLib) -> Result<ArrayedCell> {
    let mut layout = Layout {
        name: params.name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let spacing = params.spacing.unwrap_or_else(|| {
        let cell = params.cell.read().unwrap();
        let bbox = cell.layout.as_ref().unwrap().bbox();
        match params.direction {
            Dir::Horiz => bbox.width(),
            Dir::Vert => bbox.height(),
        }
    });

    let has_abstract = { params.cell.read().unwrap().has_abstract() };

    let mut abs = if has_abstract {
        Some(Abstract::new(params.name.clone()))
    } else {
        None
    };

    for i in 0..params.num {
        let loc = match params.direction {
            Dir::Horiz => Point::new(spacing * i as isize, 0),
            Dir::Vert => Point::new(0, spacing * i as isize),
        };

        let mut inst = Instance {
            inst_name: format!("cell_{}", i),
            cell: params.cell.clone(),
            loc,
            reflect_vert: false,
            angle: None,
        };

        if (i % 2 == 0) ^ params.flip_toggle {
            match params.flip {
                FlipMode::AlternateFlipHorizontal => {
                    inst.reflect_horiz_anchored();
                }
                FlipMode::AlternateFlipVertical => {
                    inst.reflect_vert_anchored();
                }
                _ => {}
            }
        }

        if let Some(ref mut abs) = abs.as_mut() {
            let mut ports = inst.ports();
            for p in ports.iter_mut() {
                p.net = format!("{}_{}", &p.net, i);
            }
            for port in ports {
                abs.add_port(port);
            }
        }

        layout.insts.push(inst);
    }

    let cell = Cell {
        name: params.name,
        abs,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ArrayedCell { cell: ptr })
}

pub fn draw_bitcell_array(
    rows: usize,
    cols: usize,
    dummy_rows: usize,
    dummy_cols: usize,
    lib: &mut PdkLib,
) -> Result<Ptr<Cell>> {
    let name = "sram_core".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let mut abs = Abstract::new(name.clone());

    let corner = corner_gds(lib)?;
    let colend_cent = colend_cent_gds(lib)?;
    let colend_p_cent = colend_p_cent_gds(lib)?;
    let colend = colend_gds(lib)?;
    let colend_bbox = bbox(&colend);

    let bitcell = sram_sp_cell_gds(lib)?;
    let rowend = rowend_gds(lib)?;
    let wlstrap = wlstrap_gds(lib)?;
    let wlstrap_p = wlstrap_p_gds(lib)?;
    assert_eq!(colend_bbox.width(), 1200);

    let mut grid = GridCells::new();
    let mut row = vec![
        Instance {
            inst_name: "corner_ul".to_string(),
            cell: corner.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: Some(180f64),
        },
        Instance {
            inst_name: "colend_top_0".to_string(),
            cell: colend.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        },
    ];

    let total_rows = rows + 2 * dummy_rows;
    let total_cols = cols + 2 * dummy_cols;

    for i in 1..total_cols {
        let colend_cent_i = if i % 2 == 0 {
            colend_cent.clone()
        } else {
            colend_p_cent.clone()
        };

        row.push(Instance {
            inst_name: format!("colend_cent_top_{}", i),
            cell: colend_cent_i,
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: None,
        });
        row.push(Instance {
            inst_name: format!("colend_top_{}", i),
            cell: colend.clone(),
            loc: Point::new(0, 0),
            reflect_vert: i % 2 != 0,
            angle: if i % 2 != 0 { Some(180f64) } else { None },
        });
    }

    row.push(Instance {
        inst_name: "corner_ur".to_string(),
        cell: corner.clone(),
        loc: Point::new(0, 0),
        reflect_vert: false,
        angle: None,
    });

    grid.add_row(row);

    for r in 0..total_rows {
        let mut row = Vec::new();

        row.push(Instance {
            inst_name: format!("rowend_l_{}", r),
            cell: rowend.clone(),
            loc: Point::new(0, 0),
            reflect_vert: r % 2 != 0,
            angle: Some(180f64),
        });

        row.push(Instance {
            inst_name: format!("cell_{}_0", r),
            cell: bitcell.clone(),
            loc: Point::new(0, 0),
            reflect_vert: r % 2 != 0,
            angle: Some(180f64),
        });

        for c in 1..total_cols {
            let strap = if c % 2 == 0 {
                wlstrap.clone()
            } else {
                wlstrap_p.clone()
            };
            row.push(Instance {
                inst_name: format!("wlstrap_{}_{}", r, c),
                cell: strap,
                loc: Point::new(0, 0),
                reflect_vert: r % 2 == 0,
                angle: None,
            });

            let (reflect_vert, angle) = match (r % 2, c % 2) {
                (0, 0) => (false, Some(180f64)),
                (0, 1) => (true, None),
                (1, 0) => (true, Some(180f64)),
                (1, 1) => (false, None),
                _ => unreachable!("invalid mods"),
            };

            row.push(Instance {
                inst_name: format!("cell_{}_{}", r, c),
                cell: bitcell.clone(),
                loc: Point::new(0, 0),
                reflect_vert,
                angle,
            });
        }

        row.push(Instance {
            inst_name: format!("rowend_r_{}", r),
            cell: rowend.clone(),
            loc: Point::new(0, 0),
            reflect_vert: r % 2 == 0,
            angle: None,
        });

        grid.add_row(row);
    }

    let mut row = vec![
        Instance {
            inst_name: "corner_bl".to_string(),
            cell: corner.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: Some(180f64),
        },
        Instance {
            inst_name: "colend_bot_0".to_string(),
            cell: colend.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
    ];

    for i in 1..total_cols {
        let colend_cent_i = if i % 2 == 0 {
            colend_cent.clone()
        } else {
            colend_p_cent.clone()
        };

        row.push(Instance {
            inst_name: format!("colend_cent_bot_{}", i),
            cell: colend_cent_i,
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        });

        row.push(Instance {
            inst_name: format!("colend_bot_{}", i),
            cell: colend.clone(),
            loc: Point::new(0, 0),
            reflect_vert: i % 2 == 0,
            angle: if i % 2 != 0 { Some(180f64) } else { None },
        });
    }

    row.push(Instance {
        inst_name: "corner_br".to_string(),
        cell: corner,
        loc: Point::new(0, 0),
        reflect_vert: true,
        angle: None,
    });

    grid.add_row(row);

    grid.place();

    for i in 1..total_rows + 1 {
        let inst = grid.grid().get(total_rows + 1 - i, 0).unwrap();
        if inst.has_abstract() {
            for mut port in inst.ports() {
                if i == 0 || i == total_rows + 1 {
                    port.set_net(format!("{}_corner_{}_{}", &port.net, total_rows + 1 - i, 0));
                } else if i < dummy_rows + 1 || i > rows + dummy_rows {
                    let dummy_i = if i < dummy_rows + 1 { i } else { i - rows };
                    port.set_net(format!("{}_dummy_{}", &port.net, dummy_i));
                } else {
                    port.set_net(format!("{}_{}", &port.net, i - dummy_rows - 1));
                }
                abs.add_port(port);
            }
        }
    }

    for j in vec![0, total_rows + 1].into_iter() {
        let mut vpwr_counter = 0;
        let mut vgnd_counter = 0;
        let top_str = if j == 0 { "top_" } else { "" };
        for instance_i in 0..2 * total_cols + 1 {
            let inst = grid.grid().get(j, instance_i).unwrap();
            if inst.has_abstract() {
                for mut port in inst.ports() {
                    let i = (instance_i + 1) / 2;
                    if instance_i == 0 || instance_i == 2 * total_cols {
                        port.set_net(format!("{}_corner_{}_{}", &port.net, total_rows + 1, i));
                        if port.net.starts_with("vpwr0") {
                            port.set_net(format!(
                                "vpwr{}_{}{}",
                                vpwr_counter % 2,
                                top_str,
                                vpwr_counter / 2,
                            ));
                            vpwr_counter += 1;
                        }
                    } else if port.net.starts_with("vgnd") {
                        port.set_net(format!(
                            "vgnd{}_{}{}",
                            vgnd_counter % 2,
                            top_str,
                            vgnd_counter / 2
                        ));
                        vgnd_counter += 1;
                    } else if port.net.starts_with("vpwr") {
                        port.set_net(format!(
                            "vpwr{}_{}{}",
                            vpwr_counter % 2,
                            top_str,
                            vpwr_counter / 2
                        ));
                        vpwr_counter += 1;
                    } else if i < dummy_cols + 1 || i > cols + dummy_cols {
                        let dummy_i = if i < dummy_cols + 1 { i } else { i - cols };
                        port.set_net(format!("{}_dummy_{}_{}", &port.net, j, dummy_i));
                    } else if j == 1 {
                        port.set_net(format!("{}_top_{}", &port.net, i - dummy_cols - 1));
                    } else {
                        port.set_net(format!("{}_{}", &port.net, i - dummy_cols - 1));
                    }
                    println!("port net {}", &port.net);
                    abs.add_port(port);
                }
            }
        }
    }

    layout.insts = grid.into_instances();

    let cell = Cell {
        name,
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_power_connector(lib: &mut PdkLib, array: &Instance) -> Result<Ptr<Cell>> {
    let mut cell = Cell::empty("sram_array_power_connector");
    let mut router = Router::new("sram_array_power_connector_route", lib.pdk.clone());
    let cfg = router.cfg();
    let m1 = cfg.layerkey(1);
    let m2 = cfg.layerkey(2);

    let bounds = array.bbox().into_rect();

    let mut vert_ports_to_coalesce: HashMap<String, Vec<Rect>> = HashMap::new();
    for net in ["vpwr", "vgnd", "vnb", "vpb", "bl0_dummy", "bl1_dummy"] {
        for (_, port) in array.ports_starting_with(net).into_iter().enumerate() {
            if let Some(rect) = port.largest_rect(m1) {
                let mut trace = router.trace(rect, 1);
                trace.set_width(rect.width()).place_cursor_centered();
                if rect.center().y < bounds.center().y {
                    trace.vert_to(bounds.bottom() - 3_000);
                } else {
                    trace.vert_to(bounds.top() + 3_000);
                }
                if port.net.starts_with("bl") {
                    let new_net = format!("vpb_dummy_bl_{}", &port.net[9..]);
                    vert_ports_to_coalesce
                        .entry(new_net)
                        .or_insert(Vec::new())
                        .push(trace.rect());
                } else if port.net.starts_with("vgnd") {
                    let new_net = format!("vgnd{}", &port.net[5..]);
                    println!("new net {}", new_net);
                    vert_ports_to_coalesce
                        .entry(new_net)
                        .or_insert(Vec::new())
                        .push(trace.rect());
                } else if port.net.starts_with("vpwr") {
                    let new_net = format!("vpwr{}", &port.net[5..]);
                    println!("new net {}", new_net);
                    vert_ports_to_coalesce
                        .entry(new_net)
                        .or_insert(Vec::new())
                        .push(trace.rect());
                } else {
                    cell.add_pin(port.net, m1, trace.rect());
                }
            }
        }
    }

    // Merge ports that are tied to the same power straps but are too close together to have side
    // by side vias (in this case, the bitlines).
    for (net, rects) in vert_ports_to_coalesce {
        let mut final_bbox = BoundBox::empty();
        if rects.len() < 2 {
            continue;
        }
        for rect in rects {
            final_bbox = rect.union(&final_bbox);
        }
        let bbox_rect = final_bbox.into_rect();
        let trace_rect = Rect::from_spans(
            bbox_rect.hspan(),
            if bbox_rect.center().y < bounds.center().y {
                Span::new(bbox_rect.bottom(), bbox_rect.bottom() + 2_000)
            } else {
                Span::new(bbox_rect.top(), bbox_rect.top() - 2_000)
            },
        );
        let trace = router.trace(trace_rect, 1);
        cell.add_pin(net, m1, trace.rect());
    }

    let mut horiz_ports_to_coalesce: HashMap<String, Vec<Rect>> = HashMap::new();
    for net in ["vpwr", "vgnd", "vpb", "vnb", "wl_dummy"] {
        for (_, port) in array.ports_starting_with(net).into_iter().enumerate() {
            if let Some(rect) = port.largest_rect(m2) {
                let mut trace = router.trace(rect, 2);
                trace.set_width(rect.height()).place_cursor_centered();
                if rect.center().x < bounds.center().x {
                    trace.horiz_to(bounds.left() - 6_400);
                } else {
                    trace.horiz_to(bounds.right() + 6_400);
                }
                if port.net.starts_with("wl_dummy") || port.net.starts_with("vgnd_dummy") {
                    let new_net = if port.net.starts_with("wl") {
                        format!("vgnd{}", &port.net[2..])
                    } else {
                        port.net
                    };
                    match horiz_ports_to_coalesce.get_mut(&new_net) {
                        Some(rects) => {
                            rects.push(trace.rect());
                        }
                        None => {
                            horiz_ports_to_coalesce.insert(new_net, vec![trace.rect()]);
                        }
                    }
                } else if net.starts_with("vpb") {
                    cell.add_pin(format!("vpwr{}", &port.net[3..]), m2, trace.rect());
                } else if net.starts_with("vnb") {
                    cell.add_pin(format!("vgnd{}", &port.net[3..]), m2, trace.rect());
                } else {
                    cell.add_pin(port.net, m2, trace.rect());
                }
            }
        }
    }

    // Merge ports that are tied to the same power straps but are too close together to have side
    // by side vias (in this case, the wordlines).
    for (net, rects) in horiz_ports_to_coalesce {
        let mut final_bbox = BoundBox::empty();
        if rects.len() < 2 {
            continue;
        }
        for rect in rects {
            final_bbox = rect.union(&final_bbox);
        }
        let bbox_rect = final_bbox.into_rect();
        let trace_rect = Rect::from_spans(
            Span::new(bbox_rect.left(), bbox_rect.left() + 5_600),
            bbox_rect.vspan(),
        );
        let trace = router.trace(trace_rect, 2);
        cell.add_pin(net, m2, trace.rect());
    }

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
