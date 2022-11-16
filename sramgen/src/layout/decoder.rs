use crate::clog2;
use crate::layout::bank::ConnectArgs;
use crate::schematic::decoder::TreeNode;
use crate::schematic::gate::{GateParams, Size};

use crate::layout::common::bubble_ports;
use crate::layout::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{BoundBoxTrait, Cell, Instance, Int, Point, Rect, Span};
use layout21::utils::Ptr;
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::contact::ContactParams;
use pdkprims::PdkLib;
use serde::{Deserialize, Serialize};

use super::array::{draw_cell_array, ArrayCellParams, FlipMode};
use super::bank::{connect, GateList};
use super::common::MergeArgs;

use super::route::grid::{Grid, TrackLocator};
use super::route::Router;

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GateArrayParams<'a> {
    pub prefix: &'a str,
    pub width: usize,
    pub dir: Dir,
    pub pitch: Option<Int>,
}

pub fn draw_gate_array(
    lib: &mut PdkLib,
    params: GateArrayParams,
    cell: Ptr<Cell>,
    bubble_ports: &[&str],
    connect_ports: &[&str],
    skip_pins: &[&str],
) -> Result<Ptr<Cell>> {
    let GateArrayParams {
        prefix,
        width,
        dir,
        pitch,
    } = params;

    assert_eq!(dir, Dir::Vert, "Only vertical gate arrays are supported.");

    let bbox = {
        let cell = cell.read().unwrap();
        cell.layout.as_ref().unwrap().bbox()
    };

    let spacing = pitch.unwrap_or(bbox.width() + 3 * 130);

    let array = draw_cell_array(
        ArrayCellParams {
            name: format!("{}_array", prefix),
            num: width,
            cell,
            spacing: Some(spacing),
            flip: FlipMode::AlternateFlipVertical,
            flip_toggle: false,
            direction: Dir::Vert,
        },
        lib,
    )?;

    let inst = Instance::new("array", array.cell);

    let mut cell = Cell::empty(prefix);
    for prefix in bubble_ports {
        for port in inst.ports_starting_with(prefix) {
            cell.abs_mut().add_port(port);
        }
    }

    for (layer, port) in [("nwell", "vpb"), ("nsdm", "nsdm"), ("psdm", "psdm")] {
        let layer = lib.pdk.get_layerkey(layer).unwrap();
        let mut builder = MergeArgs::builder();
        builder
            .layer(layer)
            .insts(GateList::Array(&inst, width))
            .port_name(port);

        if port == "vpb" {
            // Add space for taps
            builder.right_overhang(900);
        }
        let elt = builder.build()?.element();
        cell.layout_mut().add(elt);
    }

    connect_taps_and_pwr(TapFillContext {
        lib,
        cell: &mut cell,
        prefix,
        inst: &inst,
        width,
        m1_connect_ports: connect_ports,
        skip_pins,
    })?;

    cell.layout_mut().add_inst(inst);
    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_inv_dec_array(lib: &mut PdkLib, params: GateArrayParams) -> Result<Ptr<Cell>> {
    let inv_dec = super::gate::draw_inv_dec(lib, format!("{}_inv", params.prefix))?;
    draw_gate_array(
        lib,
        params,
        inv_dec,
        &["din", "din_b"],
        &["vdd", "vss"],
        &[],
    )
}

pub fn draw_nand2_dec_array(lib: &mut PdkLib, params: GateArrayParams) -> Result<Ptr<Cell>> {
    let nand = super::gate::draw_nand2_dec(lib, format!("{}_nand", &params.prefix))?;
    draw_gate_array(lib, params, nand, &["a", "b", "y"], &["vdd", "vss"], &[])
}

pub fn draw_nand3_array(
    lib: &mut PdkLib,
    params: GateArrayParams,
    gate: GateParams,
) -> Result<Ptr<Cell>> {
    let nand = super::gate::draw_nand3(lib, gate)?;
    draw_gate_array(
        lib,
        params,
        nand,
        &["a", "b", "c", "y"],
        &["vdd0", "vdd1", "vss"],
        &["vdd1"],
    )
}

pub fn draw_and2_array(
    lib: &mut PdkLib,
    prefix: &str,
    width: usize,
    nand: GateParams,
    inv: GateParams,
) -> Result<Ptr<Cell>> {
    // TODO reduce code duplication between this and draw_and3_array.

    let nand = super::gate::draw_nand2(lib, nand)?;
    let inv = super::gate::draw_inv(lib, inv)?;

    let pitch = {
        let nand = nand.read().unwrap();
        nand.layout().bbox().height() + 240
    };

    let nand_arr = draw_gate_array(
        lib,
        GateArrayParams {
            prefix: &format!("{}_nand_array", prefix),
            width,
            dir: Dir::Vert,
            pitch: Some(pitch),
        },
        nand,
        &["a", "b", "y"],
        &["vdd", "vss"],
        &[],
    )?;
    let inv_arr = draw_gate_array(
        lib,
        GateArrayParams {
            prefix: &format!("{}_inv_array", prefix),
            width,
            dir: Dir::Vert,
            pitch: Some(pitch),
        },
        inv,
        &["din", "din_b"],
        &["vdd", "vss"],
        &[],
    )?;

    let mut cell = Cell::empty(prefix);

    let nand = Instance::new("nand_array", nand_arr);
    let nand_bbox = nand.bbox();

    let mut inv = Instance::new("inv_array", inv_arr);
    inv.align_to_the_right_of(nand_bbox, 1_000);
    inv.align_centers_vertically_gridded(nand_bbox, lib.pdk.grid());

    let mut router = Router::new(format!("{}_route", prefix), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    for i in 0..width {
        let src = nand.port(format!("y_{}", i)).largest_rect(m0).unwrap();
        let dst = inv.port(format!("din_{}", i)).largest_rect(m0).unwrap();

        let mut trace = router.trace(src, 0);
        trace.place_cursor(Dir::Horiz, true).s_bend(dst, Dir::Horiz);

        for port in ["a", "b"] {
            cell.add_pin_from_port(nand.port(format!("{}_{}", port, i)), m0);
        }
        cell.add_pin_from_port(
            nand.port(format!("y_{}", i)).named(format!("y_b_{}", i)),
            m0,
        );

        cell.add_pin_from_port(
            inv.port(format!("din_b_{}", i)).named(format!("y_{}", i)),
            m0,
        );
    }

    cell.add_pin_from_port(nand.port("vdd").named("vdd0"), m1);
    cell.add_pin_from_port(nand.port("vss").named("vss0"), m1);
    cell.add_pin_from_port(nand.port("vnb").named("vnb0"), m1);
    cell.add_pin_from_port(nand.port("vpb").named("vpb0"), m1);
    cell.add_pin_from_port(inv.port("vdd").named("vdd1"), m1);
    cell.add_pin_from_port(inv.port("vss").named("vss1"), m1);
    cell.add_pin_from_port(inv.port("vnb").named("vnb1"), m1);
    cell.add_pin_from_port(inv.port("vpb").named("vpb1"), m1);

    cell.layout_mut().add_inst(nand);
    cell.layout_mut().add_inst(inv);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_and3_array(
    lib: &mut PdkLib,
    prefix: &str,
    width: usize,
    nand: GateParams,
    inv: GateParams,
) -> Result<Ptr<Cell>> {
    let nand = super::gate::draw_nand3(lib, nand)?;
    let inv = super::gate::draw_inv(lib, inv)?;

    let pitch = {
        let nand = nand.read().unwrap();
        nand.layout().bbox().height() + 240
    };

    let nand_arr = draw_gate_array(
        lib,
        GateArrayParams {
            prefix: &format!("{}_nand_array", prefix),
            width,
            dir: Dir::Vert,
            pitch: Some(pitch),
        },
        nand,
        &["a", "b", "c", "y"],
        &["vdd0", "vdd1", "vss"],
        &["vdd1"],
    )?;
    let inv_arr = draw_gate_array(
        lib,
        GateArrayParams {
            prefix: &format!("{}_inv_array", prefix),
            width,
            dir: Dir::Vert,
            pitch: Some(pitch),
        },
        inv,
        &["din", "din_b"],
        &["vdd", "vss"],
        &[],
    )?;

    let mut cell = Cell::empty(prefix);

    let nand = Instance::new("nand_array", nand_arr);
    let nand_bbox = nand.bbox();

    let mut inv = Instance::new("inv_array", inv_arr);
    inv.align_to_the_right_of(nand_bbox, 1_000);
    inv.align_centers_vertically_gridded(nand_bbox, lib.pdk.grid());

    let mut router = Router::new(format!("{}_route", prefix), lib.pdk.clone());
    let cfg = router.cfg();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    for i in 0..width {
        let src = nand.port(format!("y_{}", i)).largest_rect(m0).unwrap();
        let dst = inv.port(format!("din_{}", i)).largest_rect(m0).unwrap();

        let mut trace = router.trace(src, 0);
        trace.place_cursor(Dir::Horiz, true).s_bend(dst, Dir::Horiz);

        for port in ["a", "b", "c"] {
            cell.add_pin_from_port(nand.port(format!("{}_{}", port, i)), m0);
        }
        cell.add_pin_from_port(
            nand.port(format!("y_{}", i)).named(format!("y_b_{}", i)),
            m0,
        );

        cell.add_pin_from_port(
            inv.port(format!("din_b_{}", i)).named(format!("y_{}", i)),
            m0,
        );
    }

    cell.add_pin_from_port(nand.port("vdd0"), m1);
    cell.add_pin_from_port(nand.port("vss").named("vss0"), m1);
    cell.add_pin_from_port(nand.port("vnb").named("vnb0"), m1);
    cell.add_pin_from_port(nand.port("vpb").named("vpb0"), m1);
    cell.add_pin_from_port(inv.port("vdd").named("vdd1"), m1);
    cell.add_pin_from_port(inv.port("vss").named("vss1"), m1);
    cell.add_pin_from_port(inv.port("vnb").named("vnb1"), m1);
    cell.add_pin_from_port(inv.port("vpb").named("vpb1"), m1);

    cell.layout_mut().add_inst(nand);
    cell.layout_mut().add_inst(inv);
    cell.layout_mut().add_inst(router.finish());

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

struct TapFillContext<'a> {
    lib: &'a mut PdkLib,
    cell: &'a mut Cell,
    prefix: &'a str,
    inst: &'a Instance,
    width: usize,
    m1_connect_ports: &'a [&'a str],
    skip_pins: &'a [&'a str],
}

fn connect_taps_and_pwr(ctx: TapFillContext) -> Result<()> {
    let TapFillContext {
        lib,
        cell,
        prefix,
        inst,
        width,
        m1_connect_ports,
        skip_pins,
    } = ctx;
    let ntapcell = draw_ntap(lib, &format!("{}_ntap", prefix))?;
    let ptapcell = draw_ptap(lib, &format!("{}_ptap", prefix))?;

    let psdm = lib.pdk.get_layerkey("psdm").unwrap();
    let nsdm = lib.pdk.get_layerkey("nsdm").unwrap();

    let mut ntaps = Vec::with_capacity(width / 2);
    let mut ptaps = Vec::with_capacity(width / 2);

    for i in 0..(width / 2) {
        let pwr1 = inst
            .port(format!("psdm_{}", 2 * i))
            .largest_rect(psdm)
            .unwrap();
        let pwr2 = inst
            .port(format!("psdm_{}", 2 * i + 1))
            .largest_rect(psdm)
            .unwrap();
        let gnd1 = inst
            .port(format!("nsdm_{}", 2 * i))
            .largest_rect(nsdm)
            .unwrap();
        let gnd2 = inst
            .port(format!("nsdm_{}", 2 * i + 1))
            .largest_rect(nsdm)
            .unwrap();

        let bbox = pwr1.bbox().union(&pwr2.bbox());
        let mut tapinst = Instance::new(format!("ntap_{}", i), ntapcell.clone());
        tapinst.align_to_the_right_of(bbox, 130);
        tapinst.align_centers_vertically_gridded(bbox, lib.pdk.grid());
        ntaps.push(tapinst);

        let bbox = gnd1.bbox().union(&gnd2.bbox());
        let mut tapinst = Instance::new(format!("ptap_{}", i), ptapcell.clone());
        tapinst.align_to_the_left_of(bbox, 130);
        tapinst.align_centers_vertically_gridded(bbox, lib.pdk.grid());
        ptaps.push(tapinst);
    }

    let mut router = Router::new(format!("{}_route", prefix), lib.pdk.clone());
    let cfg = router.cfg();
    let m1 = cfg.layerkey(1);

    let span = inst.bbox().into_rect().vspan();

    let args = ConnectArgs::builder()
        .metal_idx(1)
        .port_idx(0)
        .router(&mut router)
        .insts(GateList::Cells(&ntaps))
        .port_name("x")
        .dir(Dir::Vert)
        .span(span)
        .build()?;
    let trace = connect(args);
    cell.add_pin("vpb", m1, trace.rect());

    let args = ConnectArgs::builder()
        .metal_idx(1)
        .port_idx(0)
        .router(&mut router)
        .insts(GateList::Cells(&ptaps))
        .port_name("x")
        .dir(Dir::Vert)
        .span(span)
        .build()?;
    let trace = connect(args);
    cell.add_pin("vnb", m1, trace.rect());

    for port in m1_connect_ports {
        let args = ConnectArgs::builder()
            .metal_idx(1)
            .port_idx(0)
            .router(&mut router)
            .insts(GateList::Array(inst, width))
            .port_name(port)
            .dir(Dir::Vert)
            .span(span)
            .build()?;
        let trace = connect(args);

        if !skip_pins.contains(port) {
            cell.add_pin(*port, m1, trace.rect());
        }
    }

    cell.layout_mut().insts.append(&mut ntaps);
    cell.layout_mut().insts.append(&mut ptaps);
    cell.layout_mut().add_inst(router.finish());

    Ok(())
}

pub fn draw_ntap(lib: &mut PdkLib, _name: &str) -> Result<Ptr<Cell>> {
    let ct = lib.pdk.get_contact(
        &ContactParams::builder()
            .stack("ntap")
            .rows(1)
            .cols(1)
            .dir(Dir::Vert)
            .build()
            .unwrap(),
    );
    Ok(ct.cell.clone())
}

pub fn draw_ptap(lib: &mut PdkLib, _name: &str) -> Result<Ptr<Cell>> {
    let ct = lib.pdk.get_contact(
        &ContactParams::builder()
            .stack("ptap")
            .rows(1)
            .cols(1)
            .dir(Dir::Vert)
            .build()
            .unwrap(),
    );
    Ok(ct.cell.clone())
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum OutputDir {
    Left,
    Right,
}

impl std::ops::Not for OutputDir {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

struct NodeContext<'a> {
    prefix: &'a str,
    output_dir: OutputDir,
    ctr: &'a mut usize,
}

impl<'a> NodeContext<'a> {
    fn alloc_id(&mut self) -> usize {
        *self.ctr += 1;
        *self.ctr
    }
}

/// Generates a hierarchical decoder.
///
/// Only 2 input AND gates are supported.
pub fn draw_hier_decode(lib: &mut PdkLib, prefix: &str, node: &TreeNode) -> Result<Ptr<Cell>> {
    let mut id = 0;
    let root_ctx = NodeContext {
        prefix,
        output_dir: OutputDir::Left,
        ctr: &mut id,
    };
    draw_hier_decode_node(lib, node, root_ctx)
}

fn draw_hier_decode_node(
    lib: &mut PdkLib,
    node: &TreeNode,
    mut ctx: NodeContext,
) -> Result<Ptr<Cell>> {
    // Generate all child decoders
    let decoders = node
        .children
        .iter()
        .map(|n| {
            draw_hier_decode_node(
                lib,
                n,
                NodeContext {
                    prefix: ctx.prefix,
                    output_dir: !ctx.output_dir,
                    ctr: ctx.ctr,
                },
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let child_sizes = node.children.iter().map(|n| n.num).collect::<Vec<_>>();
    let gate_size = if !node.children.is_empty() {
        node.children.len()
    } else {
        clog2(node.num)
    };
    let bus_width: usize = child_sizes.iter().sum();

    let id = ctx.alloc_id();

    let name = format!("{}_{}", ctx.prefix, id);
    let mut cell = Cell::empty(&name);

    let nand_params = GateParams {
        name: format!("{}_nand_{}", ctx.prefix, id),
        size: Size {
            nmos_width: 3_200,
            pmos_width: 2_400,
        },
        length: 150,
    };
    let inv_params = GateParams {
        name: format!("{}_inv_{}", ctx.prefix, id),
        size: Size {
            nmos_width: 1_600,
            pmos_width: 2_400,
        },
        length: 150,
    };

    let array_name = format!("{}_{}_and_array", &ctx.prefix, id);

    let and_array = if gate_size == 2 {
        draw_and2_array(lib, &array_name, node.num, nand_params, inv_params)?
    } else if gate_size == 3 {
        draw_and3_array(lib, &array_name, node.num, nand_params, inv_params)?
    } else {
        panic!(
            "Invalid gate size: expected 2 or 3 input gate, found {}",
            gate_size
        );
    };

    let mut and_array = Instance::new("and_array", and_array);
    if ctx.output_dir == OutputDir::Left {
        and_array.reflect_horiz_anchored();
    }
    cell.layout_mut().add_inst(and_array.clone());

    let mut router = Router::new(format!("{}_{}_route", ctx.prefix, id), lib.pdk.clone());
    let cfg = router.cfg();
    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: Some(ContactPosition::CenteredAdjacent),
            below: Some(ContactPosition::CenteredNonAdjacent),
        },
    );

    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);

    for i in 0..node.num {
        cell.add_pin_from_port(
            and_array.port(format!("y_{i}")).named(format!("dec_{i}")),
            m0,
        );
        cell.add_pin_from_port(
            and_array
                .port(format!("y_b_{i}"))
                .named(format!("dec_b_{i}")),
            m0,
        );
    }

    let mut bbox = cell.layout_mut().bbox();

    let mut decoder_insts = Vec::with_capacity(decoders.len());

    for (i, decoder) in decoders.into_iter().enumerate() {
        let mut inst = Instance::new(format!("decoder_{}", i), decoder);
        inst.align_beneath(bbox, 1_270);
        cell.layout_mut().add_inst(inst.clone());
        decoder_insts.push(inst);
        bbox = cell.layout_mut().bbox();
    }

    let bbox = bbox.into_rect();
    let grid = Grid::builder()
        .center(Point::zero())
        .line(cfg.line(1))
        .space(space)
        .grid(lib.pdk.grid())
        .build()?;

    // If no child decoders, we're done.
    if bus_width == 0 {
        // Note: this only supports 2-4 and 3-8 predecoders.

        // Log2(node.num) is the number of address bits handled by this decoder.
        // The bus width is twice that, since we have addr and addr_b bits.
        let bus_width = 2 * clog2(node.num);
        // Only 2 input and 3 input gates are supported.
        assert!(bus_width == 4 || bus_width == 6);

        let track_start = match ctx.output_dir {
            OutputDir::Left => {
                grid.get_track_index(Dir::Vert, bbox.p1.x, TrackLocator::StartsBeyond) + 1
            }
            OutputDir::Right => {
                grid.get_track_index(Dir::Vert, bbox.p0.x, TrackLocator::EndsBefore)
                    - bus_width as isize
                    - 1
            }
        };
        let traces = (track_start..(track_start + bus_width as isize))
            .map(|track| {
                let rect = Rect::span_builder()
                    .with(Dir::Vert, bbox.vspan())
                    .with(Dir::Horiz, grid.vtrack(track))
                    .build();
                router.trace(rect, 1)
            })
            .collect::<Vec<_>>();

        for i in 0..node.num {
            let conns = match bus_width {
                4 => vec![("a", i % 2), ("b", 2 + (i / 2))],
                6 => vec![("a", i % 2), ("b", 2 + ((i / 2) % 2)), ("c", 4 + i / 4)],
                _ => unreachable!("bus width must be 4 or 6"),
            };
            for (port, idx) in conns {
                let src = and_array
                    .port(format!("{}_{}", port, i))
                    .largest_rect(m0)
                    .unwrap();
                let mut trace = router.trace(src, 0);
                let target = &traces[idx];
                trace
                    .place_cursor_centered()
                    .horiz_to_trace(target)
                    .contact_up(target.rect());
            }
        }

        // place ports
        for (i, trace) in traces.iter().enumerate().take(bus_width) {
            let addr_bit = i / 2;
            let addr_bar = if i % 2 == 0 { "" } else { "_b" };
            cell.add_pin(format!("addr{}_{}", addr_bar, addr_bit), m1, trace.rect())
        }

        cell.layout_mut().add_inst(router.finish());
        bubble_ports(&mut cell, &["vpb", "vnb", "vdd", "vss"], m1);

        let ptr = Ptr::new(cell);
        lib.lib.cells.push(ptr.clone());

        return Ok(ptr);
    }

    let track_start = grid.get_track_index(Dir::Vert, bbox.p1.x, TrackLocator::StartsBeyond) + 1;
    connect_subdecoders(ConnectSubdecodersArgs {
        node,
        grid: &grid,
        track_start,
        vspan: cell.layout().bbox().into_rect().vspan(),
        router: &mut router,
        gates: GateList::Array(&and_array, node.num),
        subdecoders: &decoder_insts.iter().collect::<Vec<_>>(),
    });

    // bubble up ports
    let mut addr_idx = 0;
    let mut addr_b_idx = 0;

    for decoder in decoder_insts.iter() {
        for mut port in decoder.ports().into_iter() {
            if port.net.starts_with("addr_b") {
                port.set_net(format!("addr_b_{}", addr_b_idx));
                addr_b_idx += 1;
                cell.add_pin_from_port(port, m1);
            } else if port.net.starts_with("addr") {
                port.set_net(format!("addr_{}", addr_idx));
                addr_idx += 1;
                cell.add_pin_from_port(port, m1);
            }
        }
    }

    assert_eq!(addr_idx, addr_b_idx);
    assert_eq!(2usize.pow(addr_idx), node.num);

    cell.layout_mut().add_inst(router.finish());
    bubble_ports(&mut cell, &["vpb", "vnb", "vdd", "vss"], m1);

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub(crate) struct ConnectSubdecodersArgs<'a> {
    pub(crate) node: &'a TreeNode,
    pub(crate) grid: &'a Grid,
    pub(crate) track_start: isize,
    pub(crate) vspan: Span,
    pub(crate) router: &'a mut Router,
    pub(crate) gates: GateList<'a>,
    pub(crate) subdecoders: &'a [&'a Instance],
}

pub(crate) fn bus_width(node: &TreeNode) -> usize {
    node.children.iter().map(|n| n.num).sum()
}

pub(crate) fn connect_subdecoders(args: ConnectSubdecodersArgs) {
    let child_sizes = args.node.children.iter().map(|n| n.num).collect::<Vec<_>>();
    let bus_width = bus_width(args.node);

    let cfg = args.router.cfg();
    let m0 = cfg.layerkey(0);

    let traces = (args.track_start..(args.track_start + bus_width as isize))
        .map(|track| {
            let rect = Rect::span_builder()
                .with(Dir::Vert, args.vspan)
                .with(Dir::Horiz, args.grid.vtrack(track))
                .build();
            args.router.trace(rect, 1)
        })
        .collect::<Vec<_>>();

    for i in 0..args.gates.width() {
        let mut idxs = get_idxs(i, &child_sizes);
        to_bus_idxs(&mut idxs, &child_sizes);

        assert_eq!(idxs.len(), 2);

        let ports = ["a", "b", "c", "d"]
            .into_iter()
            .take(args.node.children.len());

        // TODO generalize for 3 input gates
        for (j, port) in ports.enumerate() {
            let src = args.gates.port(port, i).largest_rect(m0).unwrap();

            let mut trace = args.router.trace(src, 0);
            let target = &traces[idxs[j]];
            trace
                .place_cursor_centered()
                .horiz_to_trace(target)
                .contact_up(target.rect());
        }
    }

    let mut base_idx = 0;

    for (decoder, node) in args.subdecoders.iter().zip(args.node.children.iter()) {
        for i in 0..node.num {
            let src = decoder.port(format!("dec_{}", i)).largest_rect(m0).unwrap();
            let mut trace = args.router.trace(src, 0);
            let target = &traces[base_idx + i];
            trace
                .place_cursor_centered()
                .horiz_to_trace(target)
                .contact_up(target.rect());
        }
        base_idx += node.num;
    }
}

pub(crate) fn get_idxs(mut num: usize, bases: &[usize]) -> Vec<usize> {
    let products = bases
        .iter()
        .rev()
        .scan(1, |state, &elem| {
            let val = *state;
            *state *= elem;
            Some(val)
        })
        .collect::<Vec<_>>();
    let mut idxs = Vec::with_capacity(bases.len());

    for i in 0..bases.len() {
        let j = products.len() - i - 1;
        idxs.push(num / products[j]);
        num %= products[j];
    }
    idxs
}

fn to_bus_idxs(idxs: &mut [usize], bases: &[usize]) {
    let mut sum = 0;
    for i in 0..idxs.len() {
        idxs[i] += sum;
        sum += bases[i];
    }
}
