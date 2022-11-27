use std::collections::HashMap;

use pdkprims::mos::MosType;

use vlsir::circuit::Module;
use vlsir::reference::To;
use vlsir::Reference;

use crate::config::col_inv::{ColInvArrayParams, ColInvParams};
use crate::schematic::conns::{
    bus, conn_slice, port_inout, port_input, port_output, sig_conn, signal,
};
use crate::schematic::mos::Mosfet;

pub fn col_inv_array(params: &ColInvArrayParams) -> Vec<Module> {
    assert!(params.width > 0);

    let inv = col_inv(&params.instance_params);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = bus("din", params.width);
    let din_b = bus("din_b", params.width);

    let ports = vec![
        port_input(&din),
        port_output(&din_b),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let mut m = Module {
        name: params.name.clone(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..params.width {
        let mut connections = HashMap::new();
        connections.insert("vdd".to_string(), sig_conn(&vdd));
        connections.insert("vss".to_string(), sig_conn(&vss));
        connections.insert("din".to_string(), conn_slice("din", i, i));
        connections.insert("din_b".to_string(), conn_slice("din_b", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("inv_{}", i),
            module: Some(Reference {
                to: Some(To::Local("col_data_inv".to_string())),
            }),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![inv, m]
}

pub fn col_inv(params: &ColInvParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let din_b = signal("din_b");

    let ports = vec![
        port_input(&din),
        port_output(&din_b),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let mut m = Module {
        name: "col_data_inv".to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "MP0".to_string(),
            width: params.pwidth,
            length,
            drain: sig_conn(&din_b),
            source: sig_conn(&vdd),
            gate: sig_conn(&din),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "MN0".to_string(),
            width: params.nwidth,
            length,
            drain: sig_conn(&din_b),
            source: sig_conn(&vss),
            gate: sig_conn(&din),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m
}
