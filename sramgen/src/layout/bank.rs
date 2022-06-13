use layout21::{
    raw::{AlignMode, Cell, Instance, Layout, Point},
    utils::Ptr,
};
use pdkprims::PdkLib;

use super::{
    array::draw_array,
    decoder::{draw_inv_dec_array, draw_nand2_array},
    dff::draw_dff_array,
    mux::{draw_read_mux_array, draw_write_mux_array},
    precharge::draw_precharge_array,
    sense_amp::draw_sense_amp_array,
    Result,
};

pub fn draw_sram_bank(rows: usize, cols: usize, lib: &mut PdkLib) -> Result<Ptr<Cell>> {
    let name = "sram_bank".to_string();

    let mut layout = Layout {
        name: name.clone(),
        insts: vec![],
        elems: vec![],
        annotations: vec![],
    };

    assert_eq!(cols % 2, 0);

    let core = draw_array(rows, cols, lib)?;
    let nand2_dec = draw_nand2_array(lib, rows)?;
    let inv_dec = draw_inv_dec_array(lib, rows)?;
    let pc = draw_precharge_array(lib, cols)?;
    let read_mux = draw_read_mux_array(lib, cols)?;
    let write_mux = draw_write_mux_array(lib, cols)?;
    let sense_amp = draw_sense_amp_array(lib, cols / 2)?;
    let dffs = draw_dff_array(lib, cols / 2)?;

    let core = Instance {
        cell: core,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "core".to_string(),
        reflect_vert: false,
    };

    let mut nand2_dec = Instance {
        cell: nand2_dec,
        loc: Point::new(0, 0),
        angle: None,
        inst_name: "nand2_dec_array".to_string(),
        reflect_vert: false,
    };

    let mut inv_dec = Instance {
        cell: inv_dec,
        inst_name: "inv_dec_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut pc = Instance {
        cell: pc,
        inst_name: "precharge_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut read_mux = Instance {
        cell: read_mux,
        inst_name: "read_mux_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut write_mux = Instance {
        cell: write_mux,
        inst_name: "write_mux_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut sense_amp = Instance {
        cell: sense_amp,
        inst_name: "sense_amp_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    let mut dffs = Instance {
        cell: dffs,
        inst_name: "dff_array".to_string(),
        reflect_vert: false,
        angle: None,
        loc: Point::new(0, 0),
    };

    inv_dec
        .align(&core, AlignMode::ToTheLeft, 1_000)
        .align(&core, AlignMode::CenterVertical, 0);
    nand2_dec
        .align(&inv_dec, AlignMode::ToTheLeft, 1_000)
        .align(&core, AlignMode::CenterVertical, 0);
    pc.align(&core, AlignMode::Beneath, 1_000)
        .align(&core, AlignMode::CenterHorizontal, 0);
    read_mux
        .align(&pc, AlignMode::Beneath, 1_000)
        .align(&core, AlignMode::CenterHorizontal, 0);
    write_mux.align(&read_mux, AlignMode::Beneath, 1_000).align(
        &core,
        AlignMode::CenterHorizontal,
        0,
    );
    sense_amp
        .align(&write_mux, AlignMode::Beneath, 1_000)
        .align(&core, AlignMode::CenterHorizontal, 0);
    dffs.align(&sense_amp, AlignMode::Beneath, 1_000)
        .align(&core, AlignMode::CenterHorizontal, 0);

    layout.insts.push(core);
    layout.insts.push(nand2_dec);
    layout.insts.push(inv_dec);
    layout.insts.push(pc);
    layout.insts.push(read_mux);
    layout.insts.push(write_mux);
    layout.insts.push(sense_amp);
    layout.insts.push(dffs);

    let cell = Cell {
        name,
        abs: None,
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use super::*;

    #[test]
    fn test_sram_bank() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sram_bank")?;
        draw_sram_bank(32, 32, &mut lib)?;

        lib.save_gds()?;

        Ok(())
    }
}
