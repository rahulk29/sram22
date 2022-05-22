use std::path::PathBuf;

use layout21::{
    gds21::GdsLibrary,
    raw::{Cell, Layers, Library},
    utils::Ptr,
};
use pdkprims::tech::sky130;
use vlsir::{circuit::ExternalModule, reference::To, QualifiedName, Reference};

use crate::{
    mos::{ext_nmos, ext_pmos},
    utils::simple_ext_module,
};

pub const SKY130_DOMAIN: &str = "sky130";
pub const SRAM_SP_CELL: &str = "sram_sp_cell";
pub const SRAM_CONTROL: &str = "sramgen_control";
pub const SRAM_SP_SENSE_AMP: &str = "sramgen_sp_sense_amp";

pub fn sram_sp_cell() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_SP_CELL,
        &["BL", "BR", "VDD", "VSS", "WL"],
    )
}

fn cell_gds(
    layers: Ptr<Layers>,
    gds_file: &str,
    cell_name: &str,
) -> Result<Ptr<Cell>, Box<dyn std::error::Error>> {
    let mut path = external_gds_path();
    path.push(gds_file);
    let lib = GdsLibrary::load(&path)?;
    let lib = Library::from_gds(&lib, Some(layers))?;

    let cell = lib
        .cells
        .iter()
        .find(|&x| {
            let x = x.read().unwrap();
            x.name == cell_name
        })
        .unwrap();

    Ok(cell.clone())
}

pub fn sram_sp_cell_gds(layers: Ptr<Layers>) -> Result<Ptr<Cell>, Box<dyn std::error::Error>> {
    cell_gds(
        layers,
        "sram_sp_cell.gds",
        "sky130_fd_bd_sram__sram_sp_cell",
    )
}

pub fn sram_sp_cell_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_SP_CELL.to_string(),
        })),
    }
}

pub fn sramgen_control() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_CONTROL,
        &[
            "clk",
            "cs",
            "we",
            "pc",
            "pc_b",
            "wl_en",
            "write_driver_en",
            "sense_en",
            "vdd",
            "vss",
        ],
    )
}

pub fn sramgen_control_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_CONTROL.to_string(),
        })),
    }
}

pub fn external_gds_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("..");
    p.push("tech/sky130/gds");
    p
}

/// Reference to a single port sense amplifier.
///
/// The SPICE subcircuit definition looks like this:
/// ```spice
/// .SUBCKT AAA_Comp_SA_sense clk inn inp outn outp VDD VSS
/// ```
pub fn sramgen_sp_sense_amp() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_SP_SENSE_AMP,
        &["clk", "inn", "inp", "outn", "outp", "VDD", "VSS"],
    )
}

pub fn sramgen_sp_sense_amp_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_SP_SENSE_AMP.to_string(),
        })),
    }
}

pub fn all_external_modules() -> Vec<ExternalModule> {
    vec![
        ext_nmos(),
        ext_pmos(),
        sram_sp_cell(),
        sramgen_control(),
        sramgen_sp_sense_amp(),
    ]
}
