use crate::decoder::{DecoderTree, TreeNode};
use crate::gate::{GateParams, Size};
use crate::layout::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, AbstractPort, Cell, Instance, Layout, Point, Rect, Shape};
use layout21::utils::Ptr;
use pdkprims::bus::{ContactPolicy, ContactPosition};
use pdkprims::PdkLib;

use super::array::{draw_cell_array, ArrayCellParams, ArrayedCell, FlipMode};
use super::gate::{draw_and2, AndParams};
use super::route::grid::{Grid, TrackLocator};
use super::route::Router;

pub fn draw_nand2_array(lib: &mut PdkLib, width: usize) -> Result<ArrayedCell> {
    let nand2 = super::gate::draw_nand2_dec(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "nand2_dec_array".to_string(),
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

pub fn draw_inv_dec_array(lib: &mut PdkLib, width: usize) -> Result<ArrayedCell> {
    let inv_dec = super::gate::draw_inv_dec(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: "inv_dec_array".to_string(),
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
pub fn draw_hier_decode(lib: &mut PdkLib, tree: &DecoderTree) -> Result<Ptr<Cell>> {
    let mut id = 0;
    let root_ctx = NodeContext {
        output_dir: OutputDir::Left,
        ctr: &mut id,
    };
    draw_hier_decode_node(lib, &tree.root, root_ctx)
}

fn draw_hier_decode_node(
    lib: &mut PdkLib,
    node: &TreeNode,
    mut ctx: NodeContext,
) -> Result<Ptr<Cell>> {
    let decoders = node
        .children
        .iter()
        .map(|n| {
            draw_hier_decode_node(
                lib,
                n,
                NodeContext {
                    output_dir: !ctx.output_dir,
                    ctr: ctx.ctr,
                },
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let child_sizes = node.children.iter().map(|n| n.num).collect::<Vec<_>>();
    let bus_width: usize = child_sizes.iter().sum();

    let id = ctx.alloc_id();

    let name = format!("hier_decode_block_{}", id);
    let mut layout = Layout::new(&name);
    let mut abs = Abstract::new(&name);

    let and_params = AndParams {
        name: format!("and_hier_decode_{}", id),
        nand: GateParams {
            name: format!("nand_hier_decode_{}", id),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 1_200,
            },
            length: 150,
        },
        inv: GateParams {
            name: format!("inv_hier_decode_{}", id),
            size: Size {
                nmos_width: 2_000,
                pmos_width: 1_200,
            },
            length: 150,
        },
    };
    let and_gate = draw_and2(lib, and_params)?;
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
        println!("Adding child decoder {}", i);
        let mut inst = Instance::new(format!("decoder_{}", i), decoder);
        inst.align_beneath(bbox, 500);
        layout.add_inst(inst.clone());
        decoder_insts.push(inst);
        bbox = layout.bbox();
    }

    let mut router = Router::new(format!("hier_decode_{}_route", id), lib.pdk.clone());
    let cfg = router.cfg();
    let space = lib.pdk.bus_min_spacing(
        1,
        cfg.line(1),
        ContactPolicy {
            above: None,
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

    // If no child decoders, we're done.
    if bus_width == 0 {
        // TODO this only supports 2 to 4 decoder nodes
        // We'll eventually want 3-8 or 4-16 decoder nodes

        // TODO make this output dir independent
        let track_start = grid.get_track_index(Dir::Vert, bbox.p0.x, TrackLocator::EndsBefore) - 4;
        let traces = (track_start..(track_start + 4))
            .map(|track| {
                let rect = Rect::span_builder()
                    .with(Dir::Vert, bbox.vspan())
                    .with(Dir::Horiz, grid.vtrack(track))
                    .build();
                router.trace(rect, 1)
            })
            .collect::<Vec<_>>();

        for (i, gate) in gates.iter().enumerate() {
            let (a, b) = (i % 2, 2 + (i / 2));
            for (port, idx) in [("A", a), ("B", b)] {
                let src = gate.port(port).largest_rect(m0).unwrap();
                let mut trace = router.trace(src, 0);
                let target = &traces[idx];
                trace
                    .place_cursor_centered()
                    .horiz_to_trace(&target)
                    .contact_up(target.rect());
            }

            let addr_bit = if i < 2 { 0 } else { 1 };
            let addr_bar = if i % 2 == 0 { "" } else { "_b" };
            let mut port = AbstractPort::new(format!("addr{}_{}", addr_bar, addr_bit));
            port.add_shape(m1, Shape::Rect(traces[i].rect()));
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
        let mut idxs = get_idxs(i, &child_sizes);
        to_bus_idxs(&mut idxs, &child_sizes);

        assert_eq!(idxs.len(), 2);

        for (j, port) in ["A", "B"].into_iter().enumerate() {
            let src = gate.port(port).largest_rect(m0).unwrap();

            let mut trace = router.trace(src, 0);
            let target = &traces[idxs[j]];
            trace
                .place_cursor_centered()
                .horiz_to_trace(target)
                .contact_up(target.rect());
        }
    }

    let mut base_idx = 0;
    let mut addr_idx = 0;
    let mut addr_b_idx = 0;

    for (decoder, node) in decoder_insts.iter().zip(node.children.iter()) {
        for i in 0..node.num {
            let src = decoder.port(format!("dec_{}", i)).largest_rect(m0).unwrap();
            let mut trace = router.trace(src, 0);
            let target = &traces[base_idx + i];
            trace
                .place_cursor_centered()
                .horiz_to_trace(target)
                .contact_up(target.rect());
        }

        // Bubble up ports
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

        base_idx += node.num;
    }

    assert_eq!(addr_idx, addr_b_idx);

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

fn get_idxs(mut num: usize, bases: &[usize]) -> Vec<usize> {
    let products = bases
        .iter()
        .rev()
        .scan(1, |state, &elem| {
            let val = *state;
            *state = *state * elem;
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

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_nand2_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_nand2_dec_array")?;
        draw_nand2_array(&mut lib, 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_inv_dec_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_inv_dec_array")?;
        draw_inv_dec_array(&mut lib, 32)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_hier_decode_4bit() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_hier_decode_4bit")?;
        let tree = DecoderTree::new(4);
        draw_hier_decode(&mut lib, &tree)?;

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
