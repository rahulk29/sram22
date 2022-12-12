use std::collections::HashMap;

use layout21::raw::geom::Dir;
use layout21::raw::{
    Abstract, BoundBox, BoundBoxTrait, Cell, Instance, Int, Layout, Point, Rect, Span,
};
use layout21::utils::Ptr;
use pdkprims::PdkLib;
use serde::{Deserialize, Serialize};

use crate::config::bitcell_array::{BitcellArrayDummyParams, BitcellArrayParams};
use crate::layout::bbox;
use crate::layout::route::Router;
use crate::layout::rows::AlignedRows;
use crate::tech::*;
use crate::{bus_bit, Result};

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

pub fn draw_cell_array(lib: &mut PdkLib, params: &ArrayCellParams) -> Result<ArrayedCell> {
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
            inst_name: bus_bit("cell", i),
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
                p.net = bus_bit(&p.net, i);
            }
            for port in ports {
                abs.add_port(port);
            }
        }

        layout.insts.push(inst);
    }

    let cell = Cell {
        name: params.name.clone(),
        abs,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ArrayedCell { cell: ptr })
}

pub fn draw_bitcell_array(lib: &mut PdkLib, params: &BitcellArrayParams) -> Result<Ptr<Cell>> {
    let &BitcellArrayParams {
        rows,
        cols,
        replica_cols,
        ..
    } = params;
    let name = &params.name;

    let &BitcellArrayDummyParams {
        top: dummy_rows_top,
        bottom: dummy_rows_bottom,
        left: dummy_cols_left,
        right: dummy_cols_right,
    } = &params.dummy_params;

    let mut layout = Layout {
        name: name.to_string(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    let mut abs = Abstract::new(name.to_string());

    let corner = corner_gds(lib)?;
    let colend_cent = colend_cent_gds(lib)?;
    let colend_p_cent = colend_p_cent_gds(lib)?;
    let colend = colend_gds(lib)?;
    let colend_bbox = bbox(&colend);

    let bitcell = sram_sp_cell_gds(lib)?;
    let bitcell_replica = sram_sp_cell_replica_gds(lib)?;
    let rowend = rowend_gds(lib)?;
    let rowend_replica = rowend_replica_gds(lib)?;
    let wlstrap = wlstrap_gds(lib)?;
    let wlstrap_p = wlstrap_p_gds(lib)?;
    assert_eq!(colend_bbox.width(), 1200);

    let cornera = cornera_gds(lib)?;
    let colenda_cent = colenda_cent_gds(lib)?;
    let colenda_p_cent = colenda_p_cent_gds(lib)?;
    let colenda = colenda_gds(lib)?;
    let colenda_bbox = bbox(&colend);

    let bitcell_opt1a = sram_sp_cell_opt1a_gds(lib)?;
    let bitcell_opt1a_replica = sram_sp_cell_opt1a_replica_gds(lib)?;
    let rowenda = rowenda_gds(lib)?;
    let rowenda_replica = rowenda_replica_gds(lib)?;
    let wlstrapa = wlstrapa_gds(lib)?;
    let wlstrapa_p = wlstrapa_p_gds(lib)?;
    assert_eq!(colenda_bbox.width(), 1200);

    let hstrap = sp_hstrap(lib)?;
    let rowend_hstrap = sp_rowend_hstrap(lib)?;
    let horiz_wlstrap = sp_horiz_wlstrap(lib)?;
    let horiz_wlstrap_p = sp_horiz_wlstrap_p(lib)?;

    let mut aligned_rows = AlignedRows::new();
    aligned_rows.grow_down();

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

    let hstrap_frequency = 8; // Place one horizontal tap row every 8 bitcell rows.
    let wlstrap_frequency = 8; // Place one wordline strap col every 8 bitcell cols.

    let core_rows = if rows > hstrap_frequency {
        rows * (hstrap_frequency + 1) / hstrap_frequency - 1
    } else {
        rows
    };
    let core_cols = if cols > wlstrap_frequency {
        cols * (wlstrap_frequency + 1) / wlstrap_frequency - 1
    } else {
        cols
    };
    let total_rows = core_rows + dummy_rows_top + dummy_rows_bottom;
    let total_cols = core_cols + dummy_cols_left + dummy_cols_right + replica_cols;

    let mut cflip = true;
    for c in 1..total_cols {
        let colend_cent_i = if !cflip {
            colend_cent.clone()
        } else {
            colend_p_cent.clone()
        };

        if is_wlstrap(
            c,
            core_cols,
            dummy_cols_left,
            replica_cols,
            wlstrap_frequency,
        ) {
            row.push(Instance {
                inst_name: format!("colend_cent_top_{}", c),
                cell: colend_cent_i,
                loc: Point::new(0, 0),
                reflect_vert: false,
                angle: None,
            });
        } else {
            row.push(Instance {
                inst_name: format!("colend_top_{}", c),
                cell: colend.clone(),
                loc: Point::new(0, 0),
                reflect_vert: cflip,
                angle: if cflip { Some(180f64) } else { None },
            });
            cflip = !cflip;
        }
    }

    row.push(Instance {
        inst_name: "corner_ur".to_string(),
        cell: corner.clone(),
        loc: Point::new(0, 0),
        reflect_vert: false,
        angle: None,
    });

    aligned_rows.add_row(row);

    let mut rflip = false;

    for r in 0..total_rows {
        let mut row = Vec::new();

        let is_dummy_row = r < dummy_rows_top || r >= core_rows + dummy_rows_top;
        let is_replica_row = replica_cols > 0 && !is_dummy_row;

        if r >= dummy_rows_top
            && r < core_rows + dummy_rows_top - 1
            && (r - dummy_rows_top) % (hstrap_frequency + 1) == hstrap_frequency - 1
        {
            row.push(Instance {
                inst_name: format!("rowend_l_{}", r),
                cell: rowend_hstrap.clone(),
                loc: Point::new(0, 0),
                reflect_vert: rflip,
                angle: Some(180f64),
            });

            row.push(Instance {
                inst_name: format!("cell_{}_0", r),
                cell: hstrap.clone(),
                loc: Point::new(0, 0),
                reflect_vert: rflip,
                angle: Some(180f64),
            });

            let mut cflip = true;
            for c in 1..total_cols {
                if is_wlstrap(
                    c,
                    core_cols,
                    dummy_cols_left,
                    replica_cols,
                    wlstrap_frequency,
                ) {
                    let strap = if !cflip {
                        horiz_wlstrap.clone()
                    } else {
                        horiz_wlstrap_p.clone()
                    };
                    row.push(Instance {
                        inst_name: format!("wlstrap_{}_{}", r, c),
                        cell: strap,
                        loc: Point::new(0, 0),
                        reflect_vert: !rflip,
                        angle: None,
                    });
                } else {
                    let (reflect_vert, angle) = match (rflip, cflip) {
                        (false, false) => (false, Some(180f64)),
                        (false, true) => (true, None),
                        (true, false) => (true, Some(180f64)),
                        (true, true) => (false, None),
                    };

                    row.push(Instance {
                        inst_name: format!("cell_{}_{}", r, c),
                        cell: hstrap.clone(),
                        loc: Point::new(0, 0),
                        reflect_vert,
                        angle,
                    });

                    cflip = !cflip;
                }
            }

            row.push(Instance {
                inst_name: format!("rowend_r_{}", r),
                cell: rowend_hstrap.clone(),
                loc: Point::new(0, 0),
                reflect_vert: !rflip,
                angle: None,
            });
        } else {
            let (rowend_r, rowend_replica_r, bitcell_r, bitcell_replica_r, wlstrap_r, wlstrap_p_r) =
                if rflip {
                    (
                        rowenda.clone(),
                        rowenda_replica.clone(),
                        bitcell_opt1a.clone(),
                        bitcell_opt1a_replica.clone(),
                        wlstrapa.clone(),
                        wlstrapa_p.clone(),
                    )
                } else {
                    (
                        rowend.clone(),
                        rowend_replica.clone(),
                        bitcell.clone(),
                        bitcell_replica.clone(),
                        wlstrap.clone(),
                        wlstrap_p.clone(),
                    )
                };

            row.push(Instance {
                inst_name: format!("rowend_l_{}", r),
                cell: if is_replica_row {
                    rowend_replica_r.clone()
                } else {
                    rowend_r.clone()
                },
                loc: Point::new(0, 0),
                reflect_vert: rflip,
                angle: Some(180f64),
            });

            row.push(Instance {
                inst_name: format!("cell_{}_0", r),
                cell: if is_replica_row {
                    bitcell_replica_r.clone()
                } else {
                    bitcell_r.clone()
                },
                loc: Point::new(0, 0),
                reflect_vert: rflip,
                angle: Some(180f64),
            });

            let mut cflip = true;
            for c in 1..total_cols {
                if is_wlstrap(
                    c,
                    core_cols,
                    dummy_cols_left,
                    replica_cols,
                    wlstrap_frequency,
                ) {
                    let strap = if !cflip {
                        wlstrap_r.clone()
                    } else {
                        wlstrap_p_r.clone()
                    };
                    row.push(Instance {
                        inst_name: format!("wlstrap_{}_{}", r, c),
                        cell: strap,
                        loc: Point::new(0, 0),
                        reflect_vert: !rflip,
                        angle: None,
                    });
                } else {
                    let (reflect_vert, angle) = match (rflip, cflip) {
                        (false, false) => (false, Some(180f64)),
                        (false, true) => (true, None),
                        (true, false) => (true, Some(180f64)),
                        (true, true) => (false, None),
                    };

                    let cell = if c < dummy_cols_left + replica_cols && is_replica_row {
                        bitcell_replica_r.clone()
                    } else {
                        bitcell_r.clone()
                    };
                    row.push(Instance {
                        inst_name: format!("cell_{}_{}", r, c),
                        cell,
                        loc: Point::new(0, 0),
                        reflect_vert,
                        angle,
                    });
                    cflip = !cflip;
                }
            }

            row.push(Instance {
                inst_name: format!("rowend_r_{}", r),
                cell: rowend_r.clone(),
                loc: Point::new(0, 0),
                reflect_vert: !rflip,
                angle: None,
            });
            rflip = !rflip;
        }

        aligned_rows.add_row(row);
    }

    let (corner_bot, colend_bot, colend_cent_bot, colend_p_cent_bot) = if (rows - 1) % 2 == 1 {
        (cornera, colenda, colenda_cent, colenda_p_cent)
    } else {
        (corner, colend, colend_cent, colend_p_cent)
    };

    let mut row = vec![
        Instance {
            inst_name: "corner_bl".to_string(),
            cell: corner_bot.clone(),
            loc: Point::new(0, 0),
            reflect_vert: false,
            angle: Some(180f64),
        },
        Instance {
            inst_name: "colend_bot_0".to_string(),
            cell: colend_bot.clone(),
            loc: Point::new(0, 0),
            reflect_vert: true,
            angle: None,
        },
    ];

    let mut cflip = true;
    for c in 1..total_cols {
        if is_wlstrap(
            c,
            core_cols,
            dummy_cols_left,
            replica_cols,
            wlstrap_frequency,
        ) {
            let colend_cent_i = if !cflip {
                colend_cent_bot.clone()
            } else {
                colend_p_cent_bot.clone()
            };

            row.push(Instance {
                inst_name: format!("colend_cent_bot_{}", c),
                cell: colend_cent_i,
                loc: Point::new(0, 0),
                reflect_vert: true,
                angle: None,
            });
        } else {
            row.push(Instance {
                inst_name: format!("colend_bot_{}", c),
                cell: colend_bot.clone(),
                loc: Point::new(0, 0),
                reflect_vert: !cflip,
                angle: if cflip { Some(180f64) } else { None },
            });
            cflip = !cflip;
        }
    }

    row.push(Instance {
        inst_name: "corner_br".to_string(),
        cell: corner_bot,
        loc: Point::new(0, 0),
        reflect_vert: true,
        angle: None,
    });

    aligned_rows.add_row(row);

    aligned_rows.place(&lib.pdk);

    // Expose ports in abstract

    // Corners
    for i in 0..4 {
        let position_str = match i {
            0 => "top_left",
            1 => "top_right",
            2 => "bottom_left",
            _ => "bottom_right",
        };
        let m = if i / 2 == 0 { 0 } else { total_rows + 1 };
        let n = if i % 2 == 0 { 0 } else { total_cols + 1 };
        let inst = aligned_rows.get(m, n);
        if inst.has_abstract() {
            for mut port in inst.ports() {
                port.set_net(format!("{}_corner_{}", &port.net, position_str));
                abs.add_port(port);
            }
        }
    }

    // Leftmost column
    for i in 1..total_rows + 1 {
        let inst = aligned_rows.get(total_rows + 1 - i, 0);
        if inst.has_abstract() {
            for mut port in inst.ports() {
                if i < dummy_rows_bottom + 1 || i > rows + dummy_rows_bottom {
                    let dummy_i = if i < dummy_rows_bottom + 1 {
                        i
                    } else {
                        i - rows - replica_cols
                    };
                    port.set_net(bus_bit(&format!("{}_dummy", &port.net), dummy_i));
                } else {
                    port.set_net(bus_bit(&port.net, i - dummy_rows_bottom - 1));
                }
                abs.add_port(port);
            }
        }
    }

    // Top and bottom rows
    for j in vec![0, total_rows + 1].into_iter() {
        let top_str = if j == 0 { "_top" } else { "" };
        for instance_i in 1..=total_cols + 1 {
            let inst = aligned_rows.get(j, instance_i);
            if inst.has_abstract() {
                for mut port in inst.ports() {
                    let i = (instance_i + 1) / 2;
                    let new_net =
                        if i < dummy_cols_left + 1 || i > cols + dummy_cols_left + replica_cols {
                            format!("{}_dummy", &port.net)
                        } else if i < dummy_cols_left + replica_cols + 1 {
                            if port.net.starts_with("bl") || port.net.starts_with("br") {
                                format!("r{}", &port.net)
                            } else {
                                format!("{}_replica", &port.net)
                            }
                        } else {
                            port.net.clone()
                        };
                    let i_final = if i < dummy_cols_left + 1 {
                        i - 1
                    } else if i < dummy_cols_left + replica_cols + 1 {
                        i - dummy_cols_left - 1
                    } else if i < cols + dummy_cols_left + replica_cols + 1 {
                        i - dummy_cols_left - replica_cols - 1
                    } else {
                        i - cols - replica_cols
                    };
                    port.set_net(bus_bit(&format!("{}{}", &new_net, top_str), i_final));
                    abs.add_port(port);
                }
            }
        }
    }

    layout.insts = aligned_rows.into_instances();

    let cell = Cell {
        name: name.to_string(),
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
                let top_str = if rect.center().y < bounds.center().y {
                    trace.vert_to(bounds.bottom() - 3_600);
                    ""
                } else {
                    trace.vert_to(bounds.top() + 3_600);
                    "_top"
                };
                if port.net.starts_with("bl") {
                    vert_ports_to_coalesce
                        .entry(format!("vpb_dummy_bl{}", top_str))
                        .or_default()
                        .push(trace.rect());
                } else if port.net.starts_with("vgnd") || port.net.starts_with("vnb") {
                    vert_ports_to_coalesce
                        .entry(format!("vgnd{}", top_str))
                        .or_default()
                        .push(trace.rect());
                } else if port.net.starts_with("vpwr") || port.net.starts_with("vpb") {
                    vert_ports_to_coalesce
                        .entry(format!("vpwr{}", top_str))
                        .or_default()
                        .push(trace.rect());
                } else {
                    cell.add_pin(port.net, m1, trace.rect());
                }
            }
        }
    }

    // Merge ports that are tied to the same power straps but are too close together to have side
    // by side vias.
    for (net, mut rects) in vert_ports_to_coalesce {
        rects.sort_by_key(|rect| rect.center().x);
        let mut current_bbox = BoundBox::empty();
        let mut bboxes = Vec::new();

        for rect in rects {
            if current_bbox.is_empty()
                || std::cmp::min(rect.center().x - 130, rect.left())
                    < std::cmp::max(
                        current_bbox.center().x + 130,
                        current_bbox.into_rect().right(),
                    ) + 140
            {
                current_bbox = rect.union(&current_bbox);
            } else {
                bboxes.push(current_bbox);
                current_bbox = rect.bbox();
            }
        }
        if !current_bbox.is_empty() {
            bboxes.push(current_bbox);
        }

        for (i, bbox) in bboxes.into_iter().enumerate() {
            let bbox_rect = bbox.into_rect();
            let trace_rect = Rect::from_spans(
                bbox_rect.hspan(),
                if bbox_rect.center().y < bounds.center().y {
                    Span::new(bbox_rect.bottom(), bbox_rect.bottom() + 3_000)
                } else {
                    Span::new(bbox_rect.top(), bbox_rect.top() - 3_000)
                },
            );
            let trace = router.trace(trace_rect, 1);
            cell.add_pin(bus_bit(&net, i), m1, trace.rect());
        }
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
                    horiz_ports_to_coalesce
                        .entry("vgnd_dummy".to_string())
                        .or_default()
                        .push(trace.rect());
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
    // by side vias.
    for (net, mut rects) in horiz_ports_to_coalesce {
        rects.sort_by_key(|rect| rect.center().y);
        let mut current_bbox = BoundBox::empty();
        let mut bboxes = Vec::new();

        for rect in rects {
            if current_bbox.is_empty()
                || std::cmp::min(rect.center().y - 130, rect.bottom())
                    < std::cmp::max(
                        current_bbox.center().y + 130,
                        current_bbox.into_rect().top(),
                    ) + 140
            {
                current_bbox = rect.union(&current_bbox);
            } else {
                bboxes.push(current_bbox);
                current_bbox = rect.bbox();
            }
        }
        if !current_bbox.is_empty() {
            bboxes.push(current_bbox);
        }

        for (i, bbox) in bboxes.into_iter().enumerate() {
            let bbox_rect = bbox.into_rect();
            let trace_rect = Rect::from_spans(
                if bbox_rect.center().x < bounds.center().x {
                    Span::new(bbox_rect.left(), bbox_rect.left() + 5_600)
                } else {
                    Span::new(bbox_rect.right(), bbox_rect.right() - 5_600)
                },
                bbox_rect.vspan(),
            );
            let trace = router.trace(trace_rect, 2);
            cell.add_pin(bus_bit(&net, i), m2, trace.rect());
        }
    }

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

fn is_wlstrap(
    c: usize,
    core_cols: usize,
    dummy_cols_left: usize,
    replica_cols: usize,
    wlstrap_frequency: usize,
) -> bool {
    c >= dummy_cols_left + replica_cols
        && c < core_cols + dummy_cols_left + replica_cols - 1
        && (c - dummy_cols_left - replica_cols) % (wlstrap_frequency + 1) == wlstrap_frequency - 1
}
