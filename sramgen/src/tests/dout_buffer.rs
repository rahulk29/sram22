use crate::layout::dout_buffer::*;
use crate::schematic::dout_buffer::*;
use crate::tech::all_external_modules;
use crate::tests::test_gds_path;
use crate::utils::save_modules;
use crate::{generate_netlist, save_bin, Result};
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_dout_buffer() -> Result<()> {
    let name = "sramgen_dout_buffer";
    let buf = dout_buf(DoutBufParams {
        length: 150,
        nw1: 1_000,
        pw1: 1_600,
        nw2: 2_000,
        pw2: 3_200,
    });
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![buf],
        ext_modules,
    };

    save_bin(name, pkg)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dout_buffer(&mut lib, name)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}

#[test]
fn test_dout_buffer_array() -> Result<()> {
    let name = "sramgen_dout_buffer_array";
    let width = 32;

    let modules = dout_buf_array(DoutBufArrayParams {
        name: name.to_string(),
        width,
        instance_params: DoutBufParams {
            length: 150,
            nw1: 1_000,
            pw1: 1_600,
            nw2: 2_000,
            pw2: 3_200,
        },
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dout_buffer_array(&mut lib, name, width as usize, 2)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
