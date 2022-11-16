use crate::generate_netlist;
use crate::schematic::gate::Size;
use crate::schematic::wl_driver::*;
use crate::utils::save_modules;

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

    save_modules(name, modules)?;

    generate_netlist(name)?;

    Ok(())
}
