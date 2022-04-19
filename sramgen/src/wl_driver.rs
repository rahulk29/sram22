use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use pdkprims::config::Int;

use vlsir::circuit::{Instance, Module};

use crate::{
    gate::{and2, AndParams, Size},
    utils::{
        bus, conn_map, conns::conn_slice, local_reference, port_inout, port_input, port_output,
        sig_conn, signal,
    },
};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WordlineDriverParams {
    pub name: String,
    pub length: Int,
    pub nand_size: Size,
    pub inv_size: Size,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WordlineDriverArrayParams {
    pub name: String,
    pub width: i64,
    pub instance_params: WordlineDriverParams,
}

pub fn wordline_driver_array(params: WordlineDriverArrayParams) -> Vec<Module> {
    assert!(params.width > 0);
    assert_eq!(params.width % 4, 0);

    let iparams = params.instance_params.clone();
    let mut wl_driver = wordline_driver(iparams);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = bus("din", params.width);
    let wl_en = signal("wl_en");
    let wl = bus("wl", params.width);

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&din),
        port_input(&wl_en),
        port_output(&wl),
    ];

    let mut m = Module {
        name: params.name,
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
        connections.insert("wl_en".to_string(), sig_conn(&wl_en));
        connections.insert("wl".to_string(), conn_slice("wl", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("wl_driver_{}", i),
            module: local_reference(&params.instance_params.name),
            parameters: HashMap::new(),
            connections,
        });
    }

    let mut modules = Vec::new();
    modules.append(&mut wl_driver);
    modules.push(m);
    modules
}

/// Drives the wordlines
pub fn wordline_driver(params: WordlineDriverParams) -> Vec<Module> {
    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let wl_en = signal("wl_en");
    let wl = signal("wl");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&din),
        port_input(&wl_en),
        port_output(&wl),
    ];

    let mut m = Module {
        name: params.name.clone(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    let and2_name = format!("{}_and2", &params.name);
    let mut and2 = and2(AndParams {
        name: and2_name.clone(),
        length: params.length,
        inv_size: params.inv_size,
        nand_size: params.nand_size,
    });

    let mut conns = HashMap::new();
    conns.insert("a", sig_conn(&din));
    conns.insert("b", sig_conn(&wl_en));
    conns.insert("y", sig_conn(&wl));
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));

    m.instances.push(Instance {
        name: "and2".to_string(),
        module: local_reference(and2_name),
        parameters: HashMap::new(),
        connections: conn_map(conns),
    });

    let mut modules = Vec::new();
    modules.append(&mut and2);
    modules.push(m);
    modules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::save_modules;

    #[test]
    fn test_netlist_and2() -> Result<(), Box<dyn std::error::Error>> {
        let modules = wordline_driver_array(WordlineDriverArrayParams {
            name: "sramgen_wordline_driver_array".to_string(),
            width: 32,
            instance_params: WordlineDriverParams {
                name: "sramgen_wordline_driver".to_string(),
                nand_size: Size {
                    nmos_width: 2_000,
                    pmos_width: 2_000,
                },
                inv_size: Size {
                    nmos_width: 1_000,
                    pmos_width: 2_000,
                },
                length: 150,
            },
        });

        save_modules("wordline_driver_array", modules)?;
        Ok(())
    }
}
