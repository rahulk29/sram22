use crate::layout::dout_buffer::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::dout_buffer::*;
use crate::schematic::{generate_netlist, save_bin, save_modules};
use crate::tech::all_external_modules;
use crate::tests::test_work_dir;
use crate::Result;
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

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dout_buffer(&mut lib, name)?;

    lib.save_gds(out_gds(&work_dir, name))?;

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

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dout_buffer_array(&mut lib, name, width as usize, 2)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
