use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, BoundBoxTrait, Cell, Instance, Int, Layout, Point};
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
        let inst = grid.grid().get(total_rows + 2 - i, 0).unwrap();
        if inst.has_abstract() {
            for mut port in inst.ports() {
                if i < dummy_rows + 1 || i > rows + dummy_rows {
                    let dummy_i = if i < dummy_rows + 1 { i } else { i - rows };
                    println!("dummy {} {} {}", &port.net, i, dummy_i);
                    port.set_net(format!("dummy_{}_{}", &port.net, dummy_i));
                } else {
                    println!("{} {} {}", &port.net, i, i - dummy_rows - 1);
                    port.set_net(format!("{}_{}", &port.net, i - dummy_rows - 1));
                }
                abs.add_port(port);
            }
        }
    }

    for instance_i in 1..2 * total_cols + 1 {
        let inst = grid.grid().get(total_rows + 1, instance_i).unwrap();
        if inst.has_abstract() {
            for mut port in inst.ports() {
                let i = (instance_i + 1) / 2;
                if i < dummy_cols + 1 || i > cols + dummy_cols {
                    let dummy_i = if i < dummy_cols + 1 {
                        i - 1
                    } else {
                        i - rows - 1
                    };
                    println!("dummy {} {} {}", &port.net, i, dummy_i);
                    port.set_net(format!("dummy_{}_{}", &port.net, dummy_i));
                } else {
                    println!("{} {} {}", &port.net, i, i - dummy_cols - 1);
                    port.set_net(format!("{}_{}", &port.net, i - dummy_cols - 1));
                }
                abs.add_port(port);
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

    for net in ["vnb", "vpb"] {
        for (i, port) in array.ports_starting_with(net).into_iter().enumerate() {
            let rect = port.largest_rect(m1).unwrap();
            let mut trace = router.trace(rect, 1);
            trace.set_width(rect.width()).place_cursor_centered();
            if rect.center().y < bounds.center().y {
                trace.vert_to(bounds.bottom() - 3_000);
            } else {
                trace.vert_to(bounds.top() + 3_000);
            }
            cell.add_pin(format!("{}_{}", net, i), m1, trace.rect());
        }
    }
    for net in ["vpwr", "vgnd"] {
        for (i, port) in array.ports_starting_with(net).into_iter().enumerate() {
            if let Some(rect) = port.largest_rect(m2) {
                let mut trace = router.trace(rect, 2);
                trace.set_width(rect.height()).place_cursor_centered();
                if rect.center().x < bounds.center().x {
                    trace.horiz_to(bounds.left() - 6_400);
                } else {
                    trace.horiz_to(bounds.right() + 6_400);
                }
                cell.add_pin(format!("{}_{}", net, i), m2, trace.rect());
            }
        }
    }

    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
