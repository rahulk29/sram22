use crate::layout::dout_buffer::*;
use crate::save_bin;
use crate::schematic::dout_buffer::*;
use crate::tech::all_external_modules;
use crate::tests::test_gds_path;
use crate::utils::save_modules;
use crate::Result;
use pdkprims::tech::sky130;
use vlsir::circuit::Package;

#[test]
fn test_dout_buffer() -> Result<()> {
    let buf = dout_buf(DoutBufParams {
        length: 150,
        nw1: 1_000,
        pw1: 1_600,
        nw2: 2_000,
        pw2: 3_200,
    });
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: "dout_buffer".to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![buf],
        ext_modules,
    };

    save_bin("dout_buffer", pkg)?;

    let mut lib = sky130::pdk_lib("test_dout_buffer")?;
    draw_dout_buffer(&mut lib, "test_dout_buffer")?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}

#[test]
fn test_dout_buffer_array() -> Result<()> {
    let width = 32;

    let modules = dout_buf_array(DoutBufArrayParams {
        name: "dout_buf_array".to_string(),
        width,
        instance_params: DoutBufParams {
            length: 150,
            nw1: 1_000,
            pw1: 1_600,
            nw2: 2_000,
            pw2: 3_200,
        },
    });

    save_modules("dout_buf_array", modules)?;

    let mut lib = sky130::pdk_lib("test_dout_buffer_array")?;
    draw_dout_buffer_array(&mut lib, "test_dout_buffer_array", width as usize, 2)?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
