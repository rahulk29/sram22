use std::collections::HashMap;

use vlsir::circuit::Module;
use vlsir::reference::To;
use vlsir::Reference;

use crate::schematic::gate::{and2, AndParams};

use crate::schematic::conns::{
    bus, conn_map, conn_slice, port_inout, port_input, port_output, sig_conn, signal,
};

#[derive(Debug, Clone)]
pub struct WriteMaskControlParams {
    pub name: String,
    pub width: i64,
    pub and_params: AndParams,
}

pub fn write_mask_control(params: WriteMaskControlParams) -> Vec<Module> {
    assert!(params.width > 0);

    let mut and = and2(params.and_params.clone());

    let vdd = signal("vdd");
    let vss = signal("vss");
    let wr_en = signal("wr_en");
    let sel = bus("sel", params.width);
    let write_driver_en = bus("write_driver_en", params.width);

    let ports = vec![
        port_input(&wr_en),
        port_input(&sel),
        port_output(&write_driver_en),
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
        let conns = [
            ("vdd", sig_conn(&vdd)),
            ("vss", sig_conn(&vss)),
            ("a", conn_slice("sel", i, i)),
            ("b", sig_conn(&wr_en)),
            ("y", conn_slice("write_driver_en", i, i)),
        ];
        m.instances.push(vlsir::circuit::Instance {
            name: format!("and2_{}", i),
            module: Some(Reference {
                to: Some(To::Local(params.and_params.name.clone())),
            }),
            parameters: HashMap::new(),
            connections: conn_map(conns.into()),
        });
    }

    let mut modules = Vec::new();
    modules.append(&mut and);
    modules.push(m);
    modules
}
