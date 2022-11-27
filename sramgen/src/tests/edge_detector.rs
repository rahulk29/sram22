use crate::paths::out_bin;
use crate::schematic::edge_detector::{edge_detector, EdgeDetectorParams};
use crate::schematic::gate::{AndParams, GateParams, Size};
use crate::schematic::{generate_netlist, save_bin};
use crate::tech::all_external_modules;
use crate::tests::test_work_dir;
use crate::Result;
use vlsir::circuit::Package;

#[test]
fn test_edge_detector() -> Result<()> {
    let name = "sramgen_edge_detector";

    let and_params = AndParams {
        name: "and2".to_string(),
        nand: GateParams {
            name: "and2_nand".to_string(),
            length: 150,
            size: Size {
                pmos_width: 2_400,
                nmos_width: 1_800,
            },
        },
        inv: GateParams {
            name: "and2_inv".to_string(),
            length: 150,
            size: Size {
                pmos_width: 2_400,
                nmos_width: 1_800,
            },
        },
    };

    let params = EdgeDetectorParams {
        prefix: name,
        num_inverters: 7,
        and_params: &and_params,
    };
    let modules = edge_detector(params);
    let ext_modules = all_external_modules();
    let pkg = Package {
        domain: name.to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules,
        ext_modules,
    };

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_bin(&bin_path, name, pkg)?;

    generate_netlist(&bin_path, &work_dir)?;

    Ok(())
}
