use crate::config::gate::{GateParams, Size};
use crate::layout::latch::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::latch::{sr_latch, SrLatchParams};
use crate::schematic::{generate_netlist, save_modules};
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;

#[test]
fn test_sr_latch() -> Result<()> {
    let name = "sramgen_sr_latch";
    let nor = GateParams {
        name: "sramgen_sr_latch_nor".to_string(),
        size: Size {
            nmos_width: 1_000,
            pmos_width: 1_600,
        },
        length: 150,
    };

    let params = SrLatchParams {
        name: name.to_string(),
        nor,
    };

    let modules = sr_latch(&params);

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sr_latch(&mut lib, &params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
