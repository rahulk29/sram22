use anyhow::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Dir, Instance, Layout, Rect, Shape, Span,
};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use crate::gate::{GateParams, Size};

use super::gate::{draw_inv, draw_nand2};
use super::route::Router;

pub fn draw_sr_latch(lib: &mut PdkLib, name: &str) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(name);
    let mut abs = Abstract::new(name);

    todo!();

    let ptr = Ptr::new(Cell {
        name: name.to_string(),
        layout: Some(layout),
        abs: Some(abs),
    });
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_path;

    use super::*;

    #[test]
    fn test_sky130_sr_latch() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_sr_latch")?;
        draw_sr_latch(&mut lib, "test_sky130_sr_latch")?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
