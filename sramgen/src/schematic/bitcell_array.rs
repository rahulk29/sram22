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
    pub dummy_rows: usize,
    pub dummy_cols: usize,
    pub name: String,
}

pub fn bitcell_array(params: BitcellArrayParams) -> Module {
    let rows = params.rows as i64;
    let cols = params.cols as i64;
    let dummy_rows = params.dummy_rows as i64;
    let dummy_cols = params.dummy_cols as i64;
    let total_rows = rows + 2 * dummy_rows;
    let total_cols = cols + 2 * dummy_rows;

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

    for i in 0..total_rows {
        for j in 0..total_cols {
            let mut connections = HashMap::new();
            connections.insert("VDD".to_string(), sig_conn(&vdd));
            connections.insert("VSS".to_string(), sig_conn(&vss));
            connections.insert("VNB".to_string(), sig_conn(&vnb));
            connections.insert("VPB".to_string(), sig_conn(&vpb));
            if i < dummy_rows
                || i > rows + dummy_rows - 1
                || j < dummy_cols
                || j > cols + dummy_cols - 1
            {
                connections.insert("BL".to_string(), sig_conn(&vss));
                connections.insert("BR".to_string(), sig_conn(&vss));
                connections.insert("WL".to_string(), sig_conn(&vss));
            } else {
                connections.insert("BL".to_string(), conn_slice("bl", j, j));
                connections.insert("BR".to_string(), conn_slice("br", j, j));
                connections.insert("WL".to_string(), conn_slice("wl", i, i));
            }
            let inst = Instance {
                name: format!("bitcell_{}_{}", i, j),
                parameters: HashMap::new(),
                module: Some(sram_sp_cell_ref()),
                connections,
            };
            m.instances.push(inst);
        }
    }

    for i in 0..total_cols {
        // .subckt sky130_fd_bd_sram__sram_sp_colend BL1 VPWR VGND BL0
        let dummy = i < dummy_rows || i > rows + dummy_rows - 1;

        let conns = [
            (
                "BL1",
                if dummy {
                    sig_conn(&vss)
                } else {
                    conn_slice("br", i, i)
                },
            ),
            (
                "BL0",
                if dummy {
                    sig_conn(&vss)
                } else {
                    conn_slice("bl", i, i)
                },
            ),
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
