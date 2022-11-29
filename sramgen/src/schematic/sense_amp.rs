use std::collections::HashMap;

use vlsir::circuit::Instance;
use vlsir::Module;

use crate::config::sense_amp::SenseAmpArrayParams;
use crate::schematic::conns::{
    bus, conn_slice, port_inout, port_input, port_output, sig_conn, signal,
};
use crate::tech::sramgen_sp_sense_amp_ref;

pub fn sense_amp_array(params: &SenseAmpArrayParams) -> Module {
    let width = params.width as i64;

    assert!(width > 0);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");
    let bl = bus("bl", width);
    let br = bus("br", width);
    let data = bus("data", width);
    let data_b = bus("data_b", width);

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&clk),
        port_input(&bl),
        port_input(&br),
        port_output(&data),
        port_output(&data_b),
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
        connections.insert("clk".to_string(), sig_conn(&clk));
        connections.insert("inn".to_string(), conn_slice("br", i, i));
        connections.insert("inp".to_string(), conn_slice("bl", i, i));
        connections.insert("outp".to_string(), conn_slice("data", i, i));
        connections.insert("outn".to_string(), conn_slice("data_b", i, i));
        connections.insert("VDD".to_string(), sig_conn(&vdd));
        connections.insert("VSS".to_string(), sig_conn(&vss));

        m.instances.push(Instance {
            name: format!("sense_amp_{}", i),
            module: Some(sramgen_sp_sense_amp_ref()),
            parameters: HashMap::new(),
            connections,
        });
    }

    m
}
