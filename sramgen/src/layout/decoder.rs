use crate::decoder::{DecoderTree, TreeNode};
use crate::gate::{GateParams, Size};
use crate::layout::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{Abstract, Cell, Instance, Layout, Point, Rect};
use layout21::utils::Ptr;
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
        gates.push(inst);
    }

    let mut bbox = layout.bbox();

    for (i, decoder) in decoders.into_iter().enumerate() {
        println!("Adding child decoder {}", i);
        let mut inst = Instance::new(format!("decoder_{}", i), decoder);
        inst.align_beneath(bbox, 500);
        layout.add_inst(inst);
        bbox = layout.bbox();
    }

    if bus_width == 0 {
        // TODO reduce copy-pasted code
        let cell = Cell {
            name,
            layout: Some(layout),
            abs: Some(abs),
        };

        let ptr = Ptr::new(cell);
        lib.lib.cells.push(ptr.clone());

        return Ok(ptr);
    }

    let mut router = Router::new(format!("hier_decode_{}_route", id), lib.pdk.clone());
    let cfg = router.cfg();
    let grid = Grid::builder()
        .center(Point::zero())
        .line(cfg.line(1))
        .space(cfg.line(1))
        .grid(lib.pdk.grid())
        .build()?;

    let track_start = grid.get_track_index(Dir::Vert, bbox.p1.x, TrackLocator::StartsBeyond);

    let bbox = bbox.into_rect();
    let m0 = cfg.layerkey(0);

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
            trace
                .place_cursor_centered()
                .horiz_to_trace(&traces[idxs[j]])
                .up();
        }
    }

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
