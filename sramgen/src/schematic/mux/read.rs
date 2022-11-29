use std::collections::HashMap;

use pdkprims::mos::MosType;

use vlsir::circuit::Module;

use crate::config::mux::{ReadMuxArrayParams, ReadMuxParams};
use crate::schematic::conns::{
    bus, conn_map, conn_slice, port_inout, port_input, sig_conn, signal,
};
use crate::schematic::local_reference;
use crate::schematic::mos::Mosfet;

pub fn read_mux_array(params: &ReadMuxArrayParams) -> Vec<Module> {
    let &ReadMuxArrayParams {
        cols, mux_ratio, ..
    } = params;
    let ReadMuxArrayParams {
        name, mux_params, ..
    } = params;

    let mux_ratio = mux_ratio as i64;
    let cols = cols as i64;

    let mux = read_mux(mux_params);
    assert!(cols > 0);
    assert_eq!(mux_ratio % 2, 0);
    assert_eq!(cols % mux_ratio, 0);

    let sel_b = bus("sel_b", mux_ratio);
    let bl = bus("bl", cols);
    let br = bus("br", cols);
    let bl_out = bus("bl_out", cols / mux_ratio);
    let br_out = bus("br_out", cols / mux_ratio);
    let vdd = signal("vdd");

    let ports = vec![
        port_input(&sel_b),
        port_inout(&bl),
        port_inout(&br),
        port_inout(&bl_out),
        port_inout(&br_out),
        port_inout(&vdd),
    ];

    let mut m = Module {
        name: name.to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..cols {
        let output_idx = i / mux_ratio;
        let sel_idx = i % mux_ratio;
        let mut connections = HashMap::new();
        connections.insert("vdd", sig_conn(&vdd));
        connections.insert("bl", conn_slice("bl", i, i));
        connections.insert("br", conn_slice("br", i, i));
        connections.insert("bl_out", conn_slice("bl_out", output_idx, output_idx));
        connections.insert("br_out", conn_slice("br_out", output_idx, output_idx));
        connections.insert("sel_b", conn_slice("sel_b", sel_idx, sel_idx));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_{}", i),
            module: local_reference(&mux_params.name),
            parameters: HashMap::new(),
            connections: conn_map(connections),
        });
    }

    vec![mux, m]
}

/// A read mux using PMOS devices
pub fn read_mux(params: &ReadMuxParams) -> Module {
    let length = params.length;

    let sel_b = signal("sel_b");
    let bl = signal("bl");
    let br = signal("br");
    let bl_out = signal("bl_out");
    let br_out = signal("br_out");
    let vdd = signal("vdd");

    let ports = vec![
        port_input(&sel_b),
        port_inout(&bl),
        port_inout(&br),
        port_inout(&bl_out),
        port_inout(&br_out),
        port_inout(&vdd),
    ];

    let mut m = Module {
        name: "column_read_mux".to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "MBL".to_string(),
            width: params.width,
            length,
            source: sig_conn(&bl),
            drain: sig_conn(&bl_out),
            gate: sig_conn(&sel_b),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.instances.push(
        Mosfet {
            name: "MBR".to_string(),
            width: params.width,
            length,
            source: sig_conn(&br),
            drain: sig_conn(&br_out),
            gate: sig_conn(&sel_b),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}
