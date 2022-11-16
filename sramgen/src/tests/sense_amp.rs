use crate::layout::sense_amp::*;
use crate::schematic::sense_amp::*;
use crate::tech::all_external_modules;
use crate::tests::test_gds_path;
use crate::{generate_netlist, save_bin, Result};
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_sky130_sense_amp_array() -> Result<()> {
    let name = "sramgen_sense_amp_array";
    let width = 16;

    let sense_amps = sense_amp_array(SenseAmpArrayParams {
        name: name.to_string(),
        width,
    });
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![sense_amps],
        ext_modules,
    };

    save_bin(name, pkg)?;

    generate_netlist(name)?;

    let mut lib = sky130::pdk_lib(name)?;
    draw_sense_amp_array(&mut lib, width as usize, 2 * 2_500)?;

    lib.save_gds(test_gds_path(name))?;

    Ok(())
}
