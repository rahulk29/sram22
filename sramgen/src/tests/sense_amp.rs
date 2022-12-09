use crate::config::sense_amp::SenseAmpArrayParams;
use crate::layout::sense_amp::*;
use crate::paths::{out_bin, out_gds};
use crate::schematic::sense_amp::*;
use crate::schematic::{generate_netlist, save_bin};
use crate::tech::all_external_modules;
use crate::tests::test_work_dir;
use crate::Result;
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_sense_amp_array() -> Result<()> {
    let name = "sramgen_sense_amp_array";

    let params = SenseAmpArrayParams {
        name: name.to_string(),
        width: 16,
        spacing: Some(2 * 2_500),
    };

    let sense_amps = sense_amp_array(&params);
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![sense_amps.into()],
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sense_amp_array(&mut lib, &params)?;

    lib.save_gds(out_gds(&work_dir, name))?;

    Ok(())
}
