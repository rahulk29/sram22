use crate::layout::sense_amp::*;
use crate::save_bin;
use crate::schematic::sense_amp::*;
use crate::tech::all_external_modules;
use crate::tests::test_gds_path;
use crate::Result;
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_sky130_sense_amp_array() -> Result<()> {
    let width = 16;

    let sense_amps = sense_amp_array(SenseAmpArrayParams {
        name: "sense_amp_array".to_string(),
        width,
    });
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: "sramgen_sense_amp_array".to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![sense_amps],
        ext_modules,
    };

    save_bin("sense_amp_array", pkg)?;

    let mut lib = sky130::pdk_lib("test_sky130_sense_amp_array")?;
    draw_sense_amp_array(&mut lib, width as usize, 2 * 2_500)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
