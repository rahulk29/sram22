use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::config::rbl::ReplicaBitcellColumnParams;
use crate::schematic::conns::{
    bus, conn_map, conn_slice, port_inout, port_input, sig_conn, signal,
};
use crate::tech::sram_sp_replica_cell_ref;

// .subckt sky130_fd_bd_sram__openram_sp_cell_opt1_replica BL BR VGND VPWR VPB VNB WL
pub fn replica_bitcell_column(params: &ReplicaBitcellColumnParams) -> Vec<Module> {
    assert!(params.rows > 0);
    let rows = params.rows as i64;
    let dummy_rows = params.dummy_rows as i64;

    let wl = bus("wl", rows);
    let rbl = signal("rbl");
    let rbr = signal("rbr");
    let vdd = signal("vdd");
    let vss = signal("vss");
    let vpb = signal("vpb");
    let vnb = signal("vnb");

    let ports = vec![
        port_input(&wl),
        port_inout(&rbl),
        port_inout(&rbr),
        port_inout(&vdd),
        port_inout(&vss),
        port_inout(&vpb),
        port_inout(&vnb),
    ];

    let mut m = Module {
        name: params.name.clone(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    let total_rows = rows + 2 * dummy_rows;

    for i in 0..total_rows {
        let mut conns = HashMap::new();
        conns.insert("BL", sig_conn(&rbl));
        conns.insert("BR", sig_conn(&rbr));
        conns.insert("VPWR", sig_conn(&vdd));
        conns.insert("VPB", sig_conn(&vpb));
        conns.insert("VGND", sig_conn(&vss));
        conns.insert("VNB", sig_conn(&vnb));

        let wl_conn = if i < dummy_rows || i >= rows + dummy_rows {
            sig_conn(&vss)
        } else {
            conn_slice("wl", i - dummy_rows, i - dummy_rows)
        };

        conns.insert("WL", wl_conn);
        m.instances.push(Instance {
            name: format!("replica_bitcell_{}", i),
            parameters: HashMap::new(),
            module: Some(sram_sp_replica_cell_ref()),
            connections: conn_map(conns),
        });
    }

    vec![m]
}
