use layout21::raw::geom::Dir;
use layout21::raw::{Cell, Element, Instance, Int};

use layout21::utils::Ptr;
use pdkprims::PdkLib;

use super::array::*;
use super::bank::GateList;
use super::common::MergeArgs;
use crate::tech::sramgen_sp_sense_amp_gds;
use crate::Result;

pub fn draw_sense_amp_array(lib: &mut PdkLib, width: usize, spacing: Int) -> Result<Ptr<Cell>> {
    let sa = sramgen_sp_sense_amp_gds(lib)?;

    let core = draw_cell_array(
        ArrayCellParams {
            name: "sense_amp_array_core".to_string(),
            num: width,
            cell: sa,
            spacing: Some(spacing),
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )?;

    let mut cell = Cell::empty("sense_amp_array");

    let inst = Instance::new("sense_amp_array_core", core.cell);

    for net in ["vdd", "vss", "clk"] {
        let rect = MergeArgs::builder()
            .layer(lib.pdk.metal(2))
            .insts(GateList::Array(&inst, width))
            .port_name(net)
            .left_overhang(100)
            .right_overhang(100)
            .build()?
            .rect();
        cell.layout_mut().add(Element {
            net: None,
            layer: lib.pdk.metal(2),
            purpose: layout21::raw::LayerPurpose::Drawing,
            inner: layout21::raw::Shape::Rect(rect),
        });
        cell.add_pin(net, lib.pdk.metal(2), rect);
    }
    for prefix in ["inp", "inn", "outp", "outn"] {
        for port in inst.ports_starting_with(prefix) {
            cell.abs_mut().add_port(port);
        }
    }

    cell.layout_mut().add_inst(inst);

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::utils::test_gds_path;

    use super::*;

    #[test]
    fn test_sky130_sense_amp_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_sense_amp_array")?;
        draw_sense_amp_array(&mut lib, 16, 2 * 2_500)?;

        lib.save_gds(test_gds_path(&lib))?;

        Ok(())
    }
}
