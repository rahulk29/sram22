use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::config::dff::DffGridParams;
use crate::schematic::conns::{
    bus, conn_slice, port_inout, port_input, port_output, sig_conn, signal,
};
use crate::tech::openram_dff_ref;

pub fn dff_grid(params: &DffGridParams) -> Vec<Module> {
    let width = (params.rows * params.cols) as i64;

    assert!(width > 0);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");

    let d = bus("d", width);
    let q = bus("q", width);
    let q_b = bus("q_b", width);

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&clk),
        port_input(&d),
        port_output(&q),
        port_output(&q_b),
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
        connections.insert("VDD".to_string(), sig_conn(&vdd));
        connections.insert("GND".to_string(), sig_conn(&vss));
        connections.insert("CLK".to_string(), sig_conn(&clk));
        connections.insert("D".to_string(), conn_slice("d", i, i));
        connections.insert("Q".to_string(), conn_slice("q", i, i));
        connections.insert("Q_N".to_string(), conn_slice("q_b", i, i));

        m.instances.push(Instance {
            name: format!("dff_{}", i),
            module: Some(openram_dff_ref()),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![m]
}
