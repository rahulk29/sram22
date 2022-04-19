use std::collections::HashMap;

use vlsir::{
    circuit::{connection::Stype, port::Direction, Connection, Instance, Port, Signal, Slice},
    Module,
};

use crate::{
    tech::sram_sp_cell_ref,
    utils::{sig_conn, signal},
};

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

    let ports = vec![
        Port {
            signal: Some(Signal {
                name: "bls".into(),
                width: cols,
            }),
            direction: Direction::Inout as i32,
        },
        Port {
            signal: Some(Signal {
                name: "brs".into(),
                width: cols,
            }),
            direction: Direction::Inout as i32,
        },
        Port {
            signal: Some(Signal {
                name: "wls".into(),
                width: rows,
            }),
            direction: Direction::Input as i32,
        },
        Port {
            signal: Some(signal("vdd")),
            direction: Direction::Inout as i32,
        },
        Port {
            signal: Some(signal("vss")),
            direction: Direction::Inout as i32,
        },
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
            connections.insert(
                "BL".to_string(),
                Connection {
                    stype: Some(Stype::Slice(Slice {
                        signal: "bls".to_string(),
                        top: j,
                        bot: j,
                    })),
                },
            );
            connections.insert(
                "BR".to_string(),
                Connection {
                    stype: Some(Stype::Slice(Slice {
                        signal: "brs".to_string(),
                        top: j,
                        bot: j,
                    })),
                },
            );
            connections.insert(
                "WL".to_string(),
                Connection {
                    stype: Some(Stype::Slice(Slice {
                        signal: "wls".to_string(),
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

    m
}

#[cfg(test)]
mod tests {
    use vlsir::circuit::Package;

    use crate::{save_bin, tech::all_external_modules};

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
}
