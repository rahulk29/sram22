use std::collections::HashMap;

use pdkprims::config::Int;
use pdkprims::mos::MosType;

use vlsir::circuit::connection::Stype;
use vlsir::circuit::{Connection, Module, Slice};
use vlsir::reference::To;
use vlsir::Reference;

use crate::schematic::conns::{bus, port_inout, port_input, sig_conn, signal};
use crate::schematic::mos::Mosfet;

#[derive(Debug, Clone)]
pub struct PrechargeParams {
    pub name: String,
    pub length: Int,
    pub pull_up_width: Int,
    pub equalizer_width: Int,
}

#[derive(Debug, Clone)]
pub struct PrechargeArrayParams {
    pub width: usize,
    pub instance_params: PrechargeParams,
    pub name: String,
}

pub fn precharge_array(params: PrechargeArrayParams) -> Vec<Module> {
    assert!(params.width > 0);

    let pc = precharge(params.instance_params.clone());

    let vdd = signal("vdd");
    let en_b = signal("en_b");
    let bl = bus("bl", params.width as i64);
    let br = bus("br", params.width as i64);

    let ports = vec![
        port_inout(&vdd),
        port_input(&en_b),
        port_inout(&bl),
        port_inout(&br),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..params.width as i64 {
        let mut connections = HashMap::new();
        connections.insert("vdd".to_string(), sig_conn(&vdd));
        connections.insert("en_b".to_string(), sig_conn(&en_b));
        connections.insert(
            "bl".to_string(),
            Connection {
                stype: Some(Stype::Slice(Slice {
                    signal: "bl".to_string(),
                    top: i,
                    bot: i,
                })),
            },
        );
        connections.insert(
            "br".to_string(),
            Connection {
                stype: Some(Stype::Slice(Slice {
                    signal: "br".to_string(),
                    top: i,
                    bot: i,
                })),
            },
        );
        m.instances.push(vlsir::circuit::Instance {
            name: format!("precharge_{}", i),
            module: Some(Reference {
                to: Some(To::Local(params.instance_params.name.clone())),
            }),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![pc, m]
}

pub fn precharge(params: PrechargeParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let bl = signal("bl");
    let br = signal("br");
    let en = signal("en_b");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&bl),
        port_inout(&br),
        port_input(&en),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "bl_pull_up".to_string(),
            width: params.pull_up_width,
            length,
            drain: sig_conn(&bl),
            source: sig_conn(&vdd),
            gate: sig_conn(&en),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.instances.push(
        Mosfet {
            name: "br_pull_up".to_string(),
            width: params.pull_up_width,
            length,
            drain: sig_conn(&br),
            source: sig_conn(&vdd),
            gate: sig_conn(&en),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "equalizer".to_string(),
            width: params.equalizer_width,
            length,
            drain: sig_conn(&bl),
            source: sig_conn(&br),
            gate: sig_conn(&en),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}
