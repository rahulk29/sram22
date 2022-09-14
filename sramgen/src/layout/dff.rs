use crate::{layout::Result, tech::openram_dff_gds};
use layout21::raw::translate::Translate;
use layout21::raw::{Abstract, Instance, Layout, Point};
use layout21::{
    raw::{Cell, Dir},
    utils::Ptr,
};
use pdkprims::PdkLib;

use crate::layout::array::*;

pub fn draw_dff_array(
    lib: &mut PdkLib,
    name: impl Into<String>,
    width: usize,
) -> Result<ArrayedCell> {
    let dff = openram_dff_gds(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: name.into(),
            num: width,
            cell: dff,
            spacing: None,
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )
}

pub fn draw_vert_dff_array(
    lib: &mut PdkLib,
    name: impl Into<String>,
    width: usize,
) -> Result<Ptr<Cell>> {
    let name = name.into();

    let mut layout = Layout::new(name.clone());
    let mut abs = Abstract::new(name.clone());

    let mut prev: Option<Instance> = None;

    let dff = openram_dff_gds(lib)?;

    let m0 = lib.pdk.metal(0);

    for i in 0..width {
        let mut inst = Instance::new(format!("dff_{}", i), dff.clone());
        if i % 2 == 0 {
            inst.reflect_vert = true;
        }

        let port = if i % 2 == 0 { "vdd" } else { "gnd" };

        if let Some(prev) = prev {
            let new_bot = inst.port(port).largest_rect(m0).unwrap().p0.y;
            let prev_bot = prev.port(port).largest_rect(m0).unwrap().p0.y;
            inst.translate(Point::new(0, prev_bot - new_bot));
        }

        let mut ports = inst.ports();
        for p in ports.iter_mut() {
            p.net = format!("{}_{}", &p.net, i);
        }
        for port in ports {
            abs.add_port(port);
        }

        layout.add_inst(inst.clone());
        prev = Some(inst);
    }

    let ptr = Ptr::new(Cell {
        name,
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
    fn test_sky130_dff_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_dff_array")?;
        draw_dff_array(&mut lib, "test_sky130_dff_array", 16)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_vert_dff_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_vert_dff_array")?;
        draw_vert_dff_array(&mut lib, "test_sky130_vert_dff_array", 8)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
