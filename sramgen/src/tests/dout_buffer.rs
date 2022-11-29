use pdkprims::tech::sky130;
use vlsir::circuit::Package;

use crate::config::dout_buffer::*;
use crate::layout::dout_buffer::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::dout_buffer::*;
use crate::schematic::{generate_netlist, save_bin, save_modules};
use crate::tech::all_external_modules;
use crate::tests::test_work_dir;
use crate::Result;

#[test]
fn test_dout_buf() -> Result<()> {
    let name = "sramgen_dout_buf";
    let params = DoutBufParams {
        name: name.to_string(),
        length: 150,
        nw1: 1_000,
        pw1: 1_600,
        nw2: 2_000,
        pw2: 3_200,
    };

    let buf = dout_buf(&params);
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
    draw_dout_buf(&mut lib, &params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}

#[test]
fn test_dout_buf_array() -> Result<()> {
    let name = "sramgen_dout_buf_array";
    let width = 32;
    let params = DoutBufArrayParams {
        name: name.to_string(),
        width,
        mux_ratio: 2,
        instance_params: DoutBufParams {
            name: "dout_buf".to_string(),
            length: 150,
            nw1: 1_000,
            pw1: 1_600,
            nw2: 2_000,
            pw2: 3_200,
        },
    };

    let modules = dout_buf_array(&params);

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_dout_buf_array(&mut lib, &params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
