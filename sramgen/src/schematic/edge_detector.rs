use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::config::gate::AndParams;
use crate::config::inv_chain::InvChainGridParams;
use crate::schematic::conns::{conn_map, port_inout, port_input, port_output, sig_conn, signal};
use crate::schematic::gate::and2;
use crate::schematic::inv_chain::inv_chain_grid;
use crate::schematic::local_reference;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct EdgeDetectorParams<'a> {
    pub prefix: &'a str,
    pub num_inverters: usize,
    pub and_params: &'a AndParams,
}

pub fn edge_detector(params: EdgeDetectorParams) -> Vec<Module> {
    assert_eq!(params.num_inverters % 2, 1);

    let EdgeDetectorParams {
        prefix,
        num_inverters,
        and_params,
    } = params;
    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let dout = signal("dout");
    let delayed = signal("delayed");

    let ports = vec![
        port_input(&din),
        port_output(&dout),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let inv_chain_name = format!("{}_invs", prefix);
    let chain = inv_chain_grid(&InvChainGridParams {
        name: inv_chain_name.clone(),
        rows: 1,
        cols: num_inverters,
    });
    let mut and2 = and2(and_params.clone());

    let mut m = Module {
        name: prefix.to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    let mut connections = HashMap::new();
    connections.insert("din", sig_conn(&din));
    connections.insert("dout", sig_conn(&delayed));
    connections.insert("vdd", sig_conn(&vdd));
    connections.insert("vss", sig_conn(&vss));

    m.instances.push(Instance {
        name: "delay_chain".to_string(),
        module: local_reference(&inv_chain_name),
        parameters: HashMap::new(),
        connections: conn_map(connections),
    });

    let mut connections = HashMap::new();
    connections.insert("a", sig_conn(&din));
    connections.insert("b", sig_conn(&delayed));
    connections.insert("y", sig_conn(&dout));
    connections.insert("vdd", sig_conn(&vdd));
    connections.insert("vss", sig_conn(&vss));

    m.instances.push(Instance {
        name: "and".to_string(),
        module: local_reference(&and_params.name),
        parameters: HashMap::new(),
        connections: conn_map(connections),
    });

    let mut modules = Vec::new();
    modules.push(chain);
    modules.append(&mut and2);
    modules.push(m);

    modules
}
