use std::collections::HashMap;
use std::path::PathBuf;

use layout21::gds21::GdsLibrary;
use layout21::raw::{Cell, Library};
use layout21::utils::Ptr;
use pdkprims::PdkLib;

use vlsir::circuit::ExternalModule;
use vlsir::reference::To;
use vlsir::{QualifiedName, Reference};

use crate::schematic::mos::{ext_nmos, ext_pmos};
use crate::utils::simple_ext_module;
use crate::NETLIST_FORMAT;

pub const SKY130_DOMAIN: &str = "sky130";
pub const SRAM_SP_CELL: &str = "sram_sp_cell";
pub const SRAM_SP_COLEND: &str = "sky130_fd_bd_sram__sram_sp_colend";
pub const SRAM_SP_REPLICA_CELL: &str = "sram_sp_replica_cell";
pub const OPENRAM_DFF: &str = "openram_dff";
pub const SRAM_CONTROL: &str = "sramgen_control";
pub const SRAM_SP_SENSE_AMP: &str = "sramgen_sp_sense_amp";

pub const BITCELL_HEIGHT: isize = 1580;
pub const BITCELL_WIDTH: isize = 1200;
pub const TAPCELL_WIDTH: isize = 1300;
pub const COLUMN_WIDTH: isize = BITCELL_WIDTH + TAPCELL_WIDTH;

#[inline]
pub fn sram_sp_cell() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_SP_CELL,
        &["BL", "BR", "VDD", "VSS", "WL", "VNB", "VPB"],
    )
}

#[inline]
pub fn sram_sp_colend() -> ExternalModule {
    // .subckt sky130_fd_bd_sram__sram_sp_colend BL1 VPWR VGND BL0
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_SP_COLEND,
        &["BL1", "VPWR", "VGND", "BL0", "VNB", "VPB"],
    )
}

#[inline]
pub fn sram_sp_replica_cell() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_SP_REPLICA_CELL,
        &["BL", "BR", "VGND", "VPWR", "VPB", "VNB", "WL"],
    )
}

#[inline]
pub fn openram_dff() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        OPENRAM_DFF,
        &["VDD", "GND", "CLK", "D", "Q", "Q_N"],
    )
}

fn name_map(lib: &Library) -> HashMap<String, Ptr<Cell>> {
    let mut map = HashMap::with_capacity(lib.cells.len());

    for cell in lib.cells.iter() {
        let icell = cell.read().unwrap();
        map.insert(icell.name.clone(), Ptr::clone(cell));
    }

    map
}

type CellGdsResult = anyhow::Result<Ptr<Cell>>;

fn cell_gds(pdk_lib: &mut PdkLib, gds_file: &str, cell_name: &str) -> CellGdsResult {
    if let Some(cell) = pdk_lib.lib.cell(cell_name) {
        return Ok(cell);
    }

    let mut path = external_gds_path();
    path.push(gds_file);
    let lib = GdsLibrary::load(&path)?;
    let lib = Library::from_gds(&lib, Some(pdk_lib.pdk.layers.clone()))?;

    let map = name_map(&pdk_lib.lib);

    for cell in lib.cells.iter() {
        let mut inner = cell.write().unwrap();
        if let Some(ref mut lay) = inner.layout {
            for inst in lay.insts.iter_mut() {
                let remap_cell = {
                    let icell = inst.cell.read().unwrap();
                    if let Some(ncell) = map.get(&icell.name) {
                        Ptr::clone(ncell)
                    } else {
                        Ptr::clone(&inst.cell)
                    }
                };
                inst.cell = remap_cell;
            }
        }
    }

    let mut t_cell = None;

    for cell in lib.cells.iter() {
        let inner = cell.read().unwrap();
        if inner.name == cell_name {
            t_cell = Some(cell);
        }

        let mut flag = false;

        for ecell in pdk_lib.lib.cells.iter() {
            let ecell = ecell.read().unwrap();
            if ecell.name == inner.name {
                flag = true;
                break;
            }
        }

        if !flag {
            pdk_lib.lib.cells.push(cell.clone());
        }
    }

    Ok(t_cell.map(Ptr::clone).unwrap())
}

pub fn openram_dff_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(lib, "openram_dff.gds", "sky130_fd_bd_sram__openram_dff")
}
pub fn sramgen_sp_sense_amp_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(lib, "sramgen_sp_sense_amp.gds", "sramgen_sp_sense_amp")
}

pub fn sc_and2_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(lib, "sc_and2_2.gds", "sky130_fd_sc_lp__and2_2")
}
pub fn sc_buf_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(lib, "sc_buf_2.gds", "sky130_fd_sc_lp__buf_2")
}
pub fn sc_inv_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(lib, "sc_inv_2.gds", "sky130_fd_sc_lp__inv_2")
}
pub fn sc_tap_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(lib, "sc_tap_2.gds", "sky130_fd_sc_lp__tap_2")
}

#[inline]
pub fn sram_sp_cell_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_cell.gds",
        "sky130_fd_bd_sram__sram_sp_cell_opt1",
    )
}

#[inline]
pub fn colend_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_colend.gds",
        "sky130_fd_bd_sram__sram_sp_colend",
    )
}

#[inline]
pub fn colend_cent_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_colend_cent.gds",
        "sky130_fd_bd_sram__sram_sp_colend_cent",
    )
}

#[inline]
pub fn colend_p_cent_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_colend_p_cent.gds",
        "sky130_fd_bd_sram__sram_sp_colend_p_cent",
    )
}

#[inline]
pub fn corner_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_corner.gds",
        "sky130_fd_bd_sram__sram_sp_corner",
    )
}

#[inline]
pub fn rowend_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_rowend.gds",
        "sky130_fd_bd_sram__sram_sp_rowend",
    )
}

#[inline]
pub fn wlstrap_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_wlstrap.gds",
        "sky130_fd_bd_sram__sram_sp_wlstrap",
    )
}

#[inline]
pub fn wlstrap_p_gds(lib: &mut PdkLib) -> CellGdsResult {
    cell_gds(
        lib,
        "sram_sp_wlstrap_p.gds",
        "sky130_fd_bd_sram__sram_sp_wlstrap_p",
    )
}

#[inline]
pub fn sram_sp_cell_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_SP_CELL.to_string(),
        })),
    }
}

#[inline]
pub fn sram_sp_colend_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_SP_COLEND.to_string(),
        })),
    }
}

#[inline]
pub fn sram_sp_replica_cell_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_SP_REPLICA_CELL.to_string(),
        })),
    }
}

#[inline]
pub fn openram_dff_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: OPENRAM_DFF.to_string(),
        })),
    }
}

#[inline]
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

#[inline]
pub fn sramgen_control_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_CONTROL.to_string(),
        })),
    }
}

#[inline]
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
#[inline]
pub fn sramgen_sp_sense_amp() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_SP_SENSE_AMP,
        &["clk", "inn", "inp", "outn", "outp", "VDD", "VSS"],
    )
}

/// Reference to the simplest control logic available.
///
/// The SPICE subcircuit definition looks like this:
/// ```spice
/// .SUBCKT sramgen_control clk we pc_b wl_en write_driver_en sense_en vdd vss
/// ```
#[inline]
pub fn sramgen_control_simple() -> ExternalModule {
    simple_ext_module(
        SKY130_DOMAIN,
        SRAM_CONTROL,
        &[
            "clk",
            "we",
            "pc_b",
            "wl_en",
            "write_driver_en",
            "sense_en",
            "vdd",
            "vss",
        ],
    )
}

#[inline]
pub fn sramgen_sp_sense_amp_ref() -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: SKY130_DOMAIN.to_string(),
            name: SRAM_SP_SENSE_AMP.to_string(),
        })),
    }
}

#[inline]
pub fn all_external_modules() -> Vec<ExternalModule> {
    vec![
        ext_nmos(NETLIST_FORMAT),
        ext_pmos(NETLIST_FORMAT),
        sram_sp_cell(),
        sram_sp_colend(),
        sram_sp_replica_cell(),
        sramgen_control_simple(),
        sramgen_sp_sense_amp(),
        openram_dff(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bbox, Result};
    use pdkprims::tech::sky130;

    #[test]
    fn test_colend() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_colend")?;
        let cell = colend_gds(&mut lib)?;
        let bbox = bbox(&cell);
        assert_eq!(bbox.width(), 1200);
        assert_eq!(bbox.height(), 2055);
        Ok(())
    }

    #[test]
    fn test_rowend() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_rowend")?;
        let cell = rowend_gds(&mut lib)?;
        let bbox = bbox(&cell);
        assert_eq!(bbox.width(), 1300);
        assert_eq!(bbox.height(), 1580);

        let cell = cell.read().unwrap();
        let abs = cell.abs.as_ref().unwrap();
        assert_eq!(abs.ports.len(), 3);
        Ok(())
    }

    #[test]
    fn test_standard_cells() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_standard_cells")?;
        sc_inv_gds(&mut lib)?;
        sc_and2_gds(&mut lib)?;
        sc_buf_gds(&mut lib)?;
        sc_tap_gds(&mut lib)?;
        Ok(())
    }
}
