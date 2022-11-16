use layout21::raw::{Cell, Element, Instance, LayerKey, LayerPurpose, Layout, Point, Rect, Shape};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use crate::tech::sram_sp_cell_gds;
use crate::Result;

pub mod array;
pub mod bank;
pub mod col_inv;
pub mod common;
pub mod control;
pub mod decoder;
pub mod dff;
pub mod dout_buffer;
pub mod gate;
pub mod grid;
pub mod guard_ring;
pub mod latch;
pub mod mux;
pub mod power;
pub mod precharge;
pub mod route;
pub mod sense_amp;
pub mod tmc;
pub mod wmask_control;

pub fn draw_bitcell(lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "t_bitcell".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    layout.insts.push(Instance {
        inst_name: "mcell".to_string(),
        cell: sram_sp_cell_gds(lib)?,
        loc: Point::new(0, 0),
        reflect_vert: false,
        angle: None,
    });

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_rect(r: Rect, layer: LayerKey) -> Element {
    Element {
        net: None,
        layer,
        inner: Shape::Rect(r),
        purpose: LayerPurpose::Drawing,
    }
}
