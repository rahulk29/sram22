use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::bus_bit;
use crate::config::inv_chain::InvChainGridParams;
use crate::schematic::conns::{conn_map, port_inout, port_input, port_output, sig_conn, signal};
use crate::tech::control_logic_inv_ref;

pub fn inv_chain_grid(params: &InvChainGridParams) -> Module {
    let &InvChainGridParams { rows, cols, .. } = params;
    let name = &params.name;
    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let dout = signal("dout");

    let ports = vec![
        port_input(&din),
        port_output(&dout),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let mut m = Module {
        name: name.to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..(rows * cols) {
        let input = if i == 0 {
            din.clone()
        } else {
            signal(bus_bit("int", i))
        };
        let output = if i == rows * cols - 1 {
            dout.clone()
        } else {
            signal(bus_bit("int", i + 1))
        };

        let mut connections = HashMap::new();
        connections.insert("din", sig_conn(&input));
        connections.insert("din_b", sig_conn(&output));
        connections.insert("vdd", sig_conn(&vdd));
        connections.insert("vss", sig_conn(&vss));

        m.instances.push(Instance {
            name: format!("inv_{}", i),
            module: Some(control_logic_inv_ref()),
            parameters: HashMap::new(),
            connections: conn_map(connections),
        });
    }

    m
}
