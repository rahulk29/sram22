use std::collections::HashMap;

use vlsir::circuit::Instance;
use vlsir::Module;

use crate::config::bitcell_array::{BitcellArrayDummyParams, BitcellArrayParams};
use crate::schematic::conns::{
    bus, conn_map, conn_slice, port_inout, port_input, sig_conn, signal,
};
use crate::tech::{sram_sp_cell_ref, sram_sp_cell_replica_ref, sram_sp_colend_ref};

pub fn bitcell_array(params: &BitcellArrayParams) -> Module {
    let rows = params.rows as i64;
    let cols = params.cols as i64;
    let replica_cols = params.replica_cols as i64;
    let dummy_params = &params.dummy_params;

    let (dummy_rows_top, dummy_rows_bottom, dummy_cols_left, dummy_cols_right) = {
        let &BitcellArrayDummyParams {
            top,
            bottom,
            left,
            right,
        } = dummy_params;
        (top as i64, bottom as i64, left as i64, right as i64)
    };

    let total_rows = rows + dummy_rows_top + dummy_rows_bottom;
    let total_cols = cols + dummy_cols_left + dummy_cols_right + replica_cols;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let bl = bus("bl", cols);
    let br = bus("br", cols);
    let wl = bus("wl", rows);
    let vnb = signal("vnb");
    let vpb = signal("vpb");
    let rbl = signal("rbl");
    let rbr = signal("rbr");

    let mut ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_inout(&bl),
        port_inout(&br),
        port_input(&wl),
        port_inout(&vnb),
        port_inout(&vpb),
    ];

    if replica_cols > 0 {
        ports.push(port_inout(&rbl));
        ports.push(port_inout(&rbr));
    }

    let mut m = Module {
        name: params.name.clone(),
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

            if i < dummy_rows_bottom || i >= rows + dummy_rows_bottom {
                connections.insert("WL".to_string(), sig_conn(&vss));
            } else {
                connections.insert(
                    "WL".to_string(),
                    conn_slice("wl", i - dummy_rows_bottom, i - dummy_rows_bottom),
                );
            }
            if j < dummy_cols_left || j >= cols + dummy_cols_left + replica_cols {
                connections.insert("BL".to_string(), sig_conn(&vdd));
                connections.insert("BR".to_string(), sig_conn(&vdd));
            } else if j < dummy_cols_left + replica_cols {
                connections.insert("BL".to_string(), sig_conn(&rbl));
                connections.insert("BR".to_string(), sig_conn(&rbr));
            } else {
                connections.insert(
                    "BL".to_string(),
                    conn_slice(
                        "bl",
                        j - dummy_cols_left - replica_cols,
                        j - dummy_cols_left - replica_cols,
                    ),
                );
                connections.insert(
                    "BR".to_string(),
                    conn_slice(
                        "br",
                        j - dummy_cols_left - replica_cols,
                        j - dummy_cols_left - replica_cols,
                    ),
                );
            }

            let module = Some(
                if j < dummy_cols_left + replica_cols
                    && i >= dummy_rows_bottom
                    && i < rows + dummy_rows_bottom
                {
                    sram_sp_cell_replica_ref()
                } else {
                    sram_sp_cell_ref()
                },
            );
            let inst = Instance {
                name: format!("bitcell_{}_{}", i, j),
                parameters: HashMap::new(),
                module,
                connections,
            };
            m.instances.push(inst);
        }
    }

    for i in 0..total_cols {
        // .subckt sky130_fd_bd_sram__sram_sp_colend BL1 VPWR VGND BL0
        let is_dummy = i < dummy_cols_left || i >= cols + dummy_cols_left + replica_cols;
        let is_replica = !is_dummy && i < dummy_cols_left + replica_cols;

        let conns = [
            (
                "BL1",
                if is_dummy {
                    sig_conn(&vdd)
                } else if is_replica {
                    sig_conn(&rbr)
                } else {
                    conn_slice(
                        "br",
                        i - dummy_cols_left - replica_cols,
                        i - dummy_cols_left - replica_cols,
                    )
                },
            ),
            (
                "BL0",
                if is_dummy {
                    sig_conn(&vdd)
                } else if is_replica {
                    sig_conn(&rbl)
                } else {
                    conn_slice(
                        "bl",
                        i - dummy_cols_left - replica_cols,
                        i - dummy_cols_left - replica_cols,
                    )
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
