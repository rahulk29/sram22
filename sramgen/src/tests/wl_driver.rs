use crate::config::gate::Size;
use crate::config::wl_driver::*;
use crate::paths::out_bin;
use crate::schematic::wl_driver::*;
use crate::schematic::{generate_netlist, save_modules};
use crate::tests::test_work_dir;

#[test]
fn test_netlist_wordline_driver_array() -> Result<(), Box<dyn std::error::Error>> {
    let name = "sramgen_wordline_driver_array";
    let modules = wordline_driver_array(WordlineDriverArrayParams {
        name: name.to_string(),
        width: 32,
        instance_params: WordlineDriverParams {
            name: "sramgen_wordline_driver".to_string(),
            nand_size: Size {
                nmos_width: 2_000,
                pmos_width: 2_000,
            },
            inv_size: Size {
                nmos_width: 1_000,
                pmos_width: 2_000,
            },
            length: 150,
        },
    });

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    Ok(())
}
