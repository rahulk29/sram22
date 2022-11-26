use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::schematic::conns::{conn_map, port_inout, port_input, port_output, sig_conn, signal};

use super::gate::{nor2, GateParams};
use super::local_reference;

pub struct SrLatchParams<'a> {
    pub name: &'a str,
    pub nor: &'a GateParams,
}

pub fn sr_latch(params: SrLatchParams) -> Vec<Module> {
    let SrLatchParams { name, nor } = params;
    let vdd = signal("vdd");
    let vss = signal("vss");
    let s = signal("s");
    let r = signal("r");
    let q = signal("q");
    let qb = signal("qb");

    let ports = vec![
        port_input(&s),
        port_input(&r),
        port_output(&q),
        port_output(&qb),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let nor = nor2(nor.to_owned());

    let mut m = Module {
        name: name.to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    let mut connections = HashMap::new();
    connections.insert("a", sig_conn(&s));
    connections.insert("b", sig_conn(&q));
    connections.insert("y", sig_conn(&qb));
    connections.insert("vdd", sig_conn(&vdd));
    connections.insert("gnd", sig_conn(&vss));

    m.instances.push(Instance {
        name: "nor_set".to_string(),
        module: local_reference(&params.nor.name),
        parameters: HashMap::new(),
        connections: conn_map(connections),
    });

    let mut connections = HashMap::new();
    connections.insert("a", sig_conn(&r));
    connections.insert("b", sig_conn(&qb));
    connections.insert("y", sig_conn(&q));
    connections.insert("vdd", sig_conn(&vdd));
    connections.insert("gnd", sig_conn(&vss));

    m.instances.push(Instance {
        name: "nor_reset".to_string(),
        module: local_reference(&params.nor.name),
        parameters: HashMap::new(),
        connections: conn_map(connections),
    });

    vec![nor, m]
}
