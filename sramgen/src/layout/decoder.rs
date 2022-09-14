use crate::clog2;
use crate::decoder::TreeNode;
use crate::gate::{GateParams, Size};
use crate::layout::bank::{ConnectArgs, M1_PWR_OVERHANG};
use crate::layout::gate::draw_and3;
use crate::layout::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, AbstractPort, Cell, Instance, Layout, Point, Rect, Shape, Span};
use layout21::utils::Ptr;
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::PdkLib;

use super::array::{draw_cell_array, ArrayCellParams, ArrayedCell, FlipMode};
use super::bank::GateList;
use super::gate::{draw_and2, AndParams};
use super::route::grid::{Grid, TrackLocator};
use super::route::Router;

pub fn draw_nand2_array(lib: &mut PdkLib, prefix: &str, width: usize) -> Result<ArrayedCell> {
    let nand2 = super::gate::draw_nand2_dec(lib, format!("{}_nand", prefix))?;

    draw_cell_array(
        ArrayCellParams {
            name: format!("{}_array", prefix),
            num: width,
            cell: nand2,
            spacing: Some(1580),
            flip: FlipMode::AlternateFlipVertical,
            flip_toggle: false,
            direction: Dir::Vert,
        },
        lib,
    )
}

pub fn draw_inv_dec_array(lib: &mut PdkLib, prefix: &str, width: usize) -> Result<ArrayedCell> {
    let inv_dec = super::gate::draw_inv_dec(lib, format!("{}_inv", prefix))?;

    draw_cell_array(
        ArrayCellParams {
            name: format!("{}_array", prefix),
            num: width,
            cell: inv_dec,
            spacing: Some(1580),
            flip: FlipMode::AlternateFlipVertical,
            flip_toggle: false,
            direction: Dir::Vert,
        },
        lib,
    )
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
    let mut layout = Layout::new(&name);
    let mut abs = Abstract::new(&name);

    let and_params = AndParams {
        name: format!("{}_and_{}", ctx.prefix, id),
        nand: GateParams {
            name: format!("{}_nand_{}", ctx.prefix, id),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 1_200,
            },
            length: 150,
        },
        inv: GateParams {
            name: format!("{}_inv_{}", ctx.prefix, id),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 1_200,
            },
            length: 150,
        },
    };

    let and_gate = if gate_size == 2 {
        draw_and2(lib, and_params)?
    } else if gate_size == 3 {
        draw_and3(lib, and_params)?
    } else {
        panic!(
            "Invalid gate size: expected 2 or 3 input gate, found {}",
            gate_size
        );
    };
    let bbox = {
        let gate = and_gate.read().unwrap();
        gate.layout.as_ref().unwrap().bbox()
    };

    let mut gates = Vec::with_capacity(node.num);
    for i in 0..node.num {
        let mut inst = Instance::new(format!("and2_{}", i), and_gate.clone());
        if ctx.output_dir == OutputDir::Left {
            inst.reflect_horiz_anchored();
        }
        inst.loc.y = i as isize * (bbox.height() + 200);
        layout.add_inst(inst.clone());
        abs.add_port(inst.port("Y").named(format!("dec_{}", i)));
        gates.push(inst);
    }

    let mut bbox = layout.bbox();

    let mut decoder_insts = Vec::with_capacity(decoders.len());

    for (i, decoder) in decoders.into_iter().enumerate() {
        let mut inst = Instance::new(format!("decoder_{}", i), decoder);
        inst.align_beneath(bbox, 500);
        layout.add_inst(inst.clone());
        decoder_insts.push(inst);
        bbox = layout.bbox();
    }

    let mut router = Router::new(format!("{}_{}_route", ctx.prefix, id), lib.pdk.clone());
    let cfg = router.cfg();
    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: Some(ContactPosition::CenteredNonAdjacent),
            below: Some(ContactPosition::CenteredNonAdjacent),
        },
    );

    let bbox = bbox.into_rect();
    let m0 = cfg.layerkey(0);
    let m1 = cfg.layerkey(1);
    let grid = Grid::builder()
        .center(Point::zero())
        .line(cfg.line(1))
        .space(space)
        .grid(lib.pdk.grid())
        .build()?;

    let ports = match gate_size {
        2 => vec!["vss0", "vdd0", "vss1", "vdd1"],
        3 => vec!["vss0", "vdd0", "vdd1", "vss1", "vdd2"],
        _ => unimplemented!(),
    };

    for port in ports {
        crate::layout::bank::connect(ConnectArgs {
            metal_idx: 1,
            port_idx: 0,
            router: &mut router,
            insts: GateList::Cells(&gates),
            port_name: port,
            dir: Dir::Vert,
            overhang: Some(M1_PWR_OVERHANG),
        });
    }

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
                grid.get_track_index(Dir::Vert, bbox.p1.x, TrackLocator::StartsBeyond)
            }
            OutputDir::Right => {
                grid.get_track_index(Dir::Vert, bbox.p0.x, TrackLocator::EndsBefore)
                    - bus_width as isize
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

        for (i, gate) in gates.iter().enumerate() {
            let conns = match bus_width {
                4 => vec![("A", i % 2), ("B", 2 + (i / 2))],
                6 => vec![("A", i % 2), ("B", 2 + ((i / 2) % 2)), ("C", 4 + i / 4)],
                _ => unreachable!("bus width must be 4 or 6"),
            };
            for (port, idx) in conns {
                let src = gate.port(port).largest_rect(m0).unwrap();
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
            let mut port = AbstractPort::new(format!("addr{}_{}", addr_bar, addr_bit));
            port.add_shape(m1, Shape::Rect(trace.rect()));
            abs.add_port(port);
        }

        layout.add_inst(router.finish());

        // TODO reduce copy-pasted code.
        let cell = Cell {
            name,
            layout: Some(layout),
            abs: Some(abs),
        };

        let ptr = Ptr::new(cell);
        lib.lib.cells.push(ptr.clone());

        return Ok(ptr);
    }

    let track_start = grid.get_track_index(Dir::Vert, bbox.p1.x, TrackLocator::StartsBeyond);
    connect_subdecoders(ConnectSubdecodersArgs {
        node,
        grid: &grid,
        track_start,
        vspan: layout.bbox().into_rect().vspan(),
        router: &mut router,
        gates: GateList::Cells(&gates),
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
            } else if port.net.starts_with("addr") {
                port.set_net(format!("addr_{}", addr_idx));
                addr_idx += 1;
            }
            abs.add_port(port);
        }
    }

    assert_eq!(addr_idx, addr_b_idx);
    assert_eq!(2usize.pow(addr_idx), node.num);

    layout.add_inst(router.finish());

    let cell = Cell {
        name,
        layout: Some(layout),
        abs: Some(abs),
    };

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

        let ports = ["A", "B", "C", "D"]
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

fn get_idxs(mut num: usize, bases: &[usize]) -> Vec<usize> {
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

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::decoder::DecoderTree;
    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_nand2_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_nand2_dec_array")?;
        draw_nand2_array(&mut lib, "nand2_dec_array", 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_inv_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_inv_dec_array")?;
        draw_inv_dec_array(&mut lib, "inv_dec_array", 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_hier_decode_4bit() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_hier_decode_4bit")?;
        let tree = DecoderTree::new(4);
        draw_hier_decode(&mut lib, "hier_decode_4b", &tree.root)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_hier_decode_5bit() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_hier_decode_5bit")?;
        let tree = DecoderTree::new(5);
        draw_hier_decode(&mut lib, "hier_decode_5b", &tree.root)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_get_idxs() {
        let bases = [4, 8, 5];
        let idxs = get_idxs(14, &bases);
        assert_eq!(idxs, [0, 2, 4]);
        let idxs = get_idxs(40, &bases);
        assert_eq!(idxs, [1, 0, 0]);
    }
}
