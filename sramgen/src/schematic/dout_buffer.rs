use std::collections::HashMap;

use pdkprims::mos::MosType;
use vlsir::circuit::Module;
use vlsir::reference::To;
use vlsir::Reference;

use crate::config::dout_buffer::{DoutBufArrayParams, DoutBufParams};
use crate::schematic::conns::{
    bus, conn_slice, port_inout, port_input, port_output, sig_conn, signal,
};
use crate::schematic::mos::Mosfet;

pub fn dout_buf_array(params: &DoutBufArrayParams) -> Vec<Module> {
    let width = params.width as i64;

    assert!(width > 0);

    let inv = dout_buf(&params.instance_params);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din1 = bus("din1", width);
    let din2 = bus("din2", width);
    let dout1 = bus("dout1", width);
    let dout2 = bus("dout2", width);

    let ports = vec![
        port_input(&din1),
        port_input(&din2),
        port_output(&dout1),
        port_output(&dout2),
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

    for i in 0..width {
        let mut connections = HashMap::new();
        connections.insert("vdd".to_string(), sig_conn(&vdd));
        connections.insert("vss".to_string(), sig_conn(&vss));
        connections.insert("din1".to_string(), conn_slice("din1", i, i));
        connections.insert("din2".to_string(), conn_slice("din2", i, i));
        connections.insert("dout1".to_string(), conn_slice("dout1", i, i));
        connections.insert("dout2".to_string(), conn_slice("dout2", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("buf_{}", i),
            module: Some(Reference {
                to: Some(To::Local("dout_buf".to_string())),
            }),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![inv, m]
}

pub fn dout_buf(params: &DoutBufParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din1 = signal("din1");
    let din2 = signal("din2");
    let dout1 = signal("dout1");
    let dout2 = signal("dout2");
    let x1 = signal("x1");
    let x2 = signal("x2");

    let ports = vec![
        port_input(&din1),
        port_input(&din2),
        port_output(&dout1),
        port_output(&dout2),
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

    for (din, x, dout, suffix) in [(&din1, &x1, &dout1, "1"), (&din2, &x2, &dout2, "2")] {
        m.instances.push(
            Mosfet {
                name: format!("MP1{}", suffix),
                width: params.pw1,
                length,
                drain: sig_conn(x),
                source: sig_conn(&vdd),
                gate: sig_conn(din),
                body: sig_conn(&vdd),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m.instances.push(
            Mosfet {
                name: format!("MN1{}", suffix),
                width: params.nw1,
                length,
                drain: sig_conn(x),
                source: sig_conn(&vss),
                gate: sig_conn(din),
                body: sig_conn(&vss),
                mos_type: MosType::Nmos,
            }
            .into(),
        );
        m.instances.push(
            Mosfet {
                name: format!("MP2{}", suffix),
                width: params.pw2,
                length,
                drain: sig_conn(dout),
                source: sig_conn(&vdd),
                gate: sig_conn(x),
                body: sig_conn(&vdd),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m.instances.push(
            Mosfet {
                name: format!("MN2{}", suffix),
                width: params.nw2,
                length,
                drain: sig_conn(dout),
                source: sig_conn(&vss),
                gate: sig_conn(x),
                body: sig_conn(&vss),
                mos_type: MosType::Nmos,
            }
            .into(),
        );
    }

    m
}
