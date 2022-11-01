use std::collections::HashMap;

use vlsir::circuit::connection::Stype;
use vlsir::circuit::{Connection, Instance, Slice};
use vlsir::Module;

use crate::tech::{sram_sp_cell_ref, sram_sp_colend_ref};
use crate::utils::conns::conn_slice;
use crate::utils::{bus, conn_map, port_inout, port_input, sig_conn, signal};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BitcellArrayParams {
    pub rows: usize,
    pub cols: usize,
    pub name: String,
}

pub fn bitcell_array(params: BitcellArrayParams) -> Module {
    let rows = params.rows as i64;
    let cols = params.cols as i64;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let bl = bus("bl", cols);
    let br = bus("br", cols);
    let wl = bus("wl", rows);
    let vnb = signal("vnb");
    let vpb = signal("vpb");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_inout(&bl),
        port_inout(&br),
        port_input(&wl),
        port_inout(&vnb),
        port_inout(&vpb),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..rows {
        for j in 0..cols {
            let mut connections = HashMap::new();
            connections.insert("VDD".to_string(), sig_conn(&signal("vdd")));
            connections.insert("VSS".to_string(), sig_conn(&signal("vss")));
            connections.insert("VNB".to_string(), sig_conn(&vnb));
            connections.insert("VPB".to_string(), sig_conn(&vpb));
            connections.insert(
                "BL".to_string(),
                Connection {
                    stype: Some(Stype::Slice(Slice {
                        signal: "bl".to_string(),
                        top: j,
                        bot: j,
                    })),
                },
            );
            connections.insert(
                "BR".to_string(),
                Connection {
                    stype: Some(Stype::Slice(Slice {
                        signal: "br".to_string(),
                        top: j,
                        bot: j,
                    })),
                },
            );
            connections.insert(
                "WL".to_string(),
                Connection {
                    stype: Some(Stype::Slice(Slice {
                        signal: "wl".to_string(),
                        top: i,
                        bot: i,
                    })),
                },
            );
            let inst = Instance {
                name: format!("bitcell_{}_{}", i, j),
                parameters: HashMap::new(),
                module: Some(sram_sp_cell_ref()),
                connections,
            };
            m.instances.push(inst);
        }
    }

    for i in 0..cols {
        // .subckt sky130_fd_bd_sram__sram_sp_colend BL1 VPWR VGND BL0
        let conns = [
            ("BL1", conn_slice("br", i, i)),
            ("BL0", conn_slice("bl", i, i)),
            ("VPWR", sig_conn(&vdd)),
            ("VGND", sig_conn(&vss)),
            ("VNB", sig_conn(&vnb)),
            ("VPB", sig_conn(&vpb)),
        ];

        let inst = Instance {
            name: format!("colend_{}_bot", i),
            parameters: HashMap::new(),
            module: Some(sram_sp_colend_ref()),
            connections: conn_map(conns.clone().into()),
        };
        m.instances.push(inst);

        let inst = Instance {
            name: format!("colend_{}_top", i),
            parameters: HashMap::new(),
            module: Some(sram_sp_colend_ref()),
            connections: conn_map(conns.into()),
        };
        m.instances.push(inst);
    }

    m
}

#[cfg(test)]
mod tests {
    use vlsir::circuit::Package;

    use crate::save_bin;
    use crate::tech::all_external_modules;

    use super::*;

    #[test]
    fn test_netlist_bitcells() -> Result<(), Box<dyn std::error::Error>> {
        let bitcells = bitcell_array(super::BitcellArrayParams {
            rows: 32,
            cols: 64,
            name: "bitcells_32x64".to_string(),
        });
        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "sramgen_bitcells".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![bitcells],
            ext_modules,
        };

        save_bin("bitcells", pkg)?;

        Ok(())
    }

    #[test]
    fn test_netlist_bitcells_2x2() -> Result<(), Box<dyn std::error::Error>> {
        let bitcells = bitcell_array(super::BitcellArrayParams {
            rows: 2,
            cols: 2,
            name: "bitcells_2x2".to_string(),
        });
        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "sramgen_bitcells_2x2".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![bitcells],
            ext_modules,
        };

        save_bin("bitcells_2x2", pkg)?;

        Ok(())
    }
}
