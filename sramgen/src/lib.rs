use decoder::DecoderTree;
use vlsir::{circuit::Package, spice::SimInput};

pub mod decoder;
pub mod gate;
pub mod mos;
pub mod netlist;
pub mod plan;
pub mod utils;

pub fn generate() -> Result<(), Box<dyn std::error::Error>> {
    let nmos = mos::ext_nmos();
    let pmos = mos::ext_pmos();

    let mut pkg = Package {
        domain: "sramgen".to_string(),
        desc: "Sramgen generated cells".to_string(),
        modules: vec![],
        ext_modules: vec![nmos, pmos],
    };

    let decoder = DecoderTree::new(5);

    let input = SimInput {
        pkg: Some(pkg),
        top: "nand2".to_string(),
        opts: None,
        an: vec![],
        ctrls: vec![],
    };

    vlsir::conv::save(&input, "hi.bin")?;
    Ok(())
}
