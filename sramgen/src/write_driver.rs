use std::collections::HashMap;

use pdkprims::{config::Int, mos::MosType};

use vlsir::{circuit::Module, reference::To, Reference};

use crate::{
    mos::Mosfet,
    utils::{bus, conns::conn_slice, port_inout, port_input, sig_conn, signal},
};

pub struct BitlineDriverParams {
    pub length: Int,
    pub width: Int,
}

pub struct BitlineDriverArrayParams {
    pub name: String,
    pub width: i64,
    pub instance_params: BitlineDriverParams,
}

pub fn bitline_driver_array(params: BitlineDriverArrayParams) -> Vec<Module> {
    assert!(params.width > 0);
    assert_eq!(params.width % 4, 0);

    let drv = bitline_driver(params.instance_params);

    let vss = signal("vss");
    let bl = bus("bl", params.width);
    let br = bus("br", params.width);
    let din = bus("din", params.width);
    let din_b = bus("din_b", params.width);
    let we = signal("we");

    let ports = vec![
        port_inout(&vss),
        port_inout(&bl),
        port_inout(&br),
        port_inout(&din),
        port_inout(&din_b),
        port_input(&we),
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
        connections.insert("vss".to_string(), sig_conn(&vss));
        connections.insert("din".to_string(), conn_slice("din", i, i));
        connections.insert("din_b".to_string(), conn_slice("din_b", i, i));
        connections.insert("bl".to_string(), conn_slice("bl", i, i));
        connections.insert("br".to_string(), conn_slice("br", i, i));
        connections.insert("we".to_string(), sig_conn(&we));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("driver_{}", i),
            module: Some(Reference {
                to: Some(To::Local("bitline_driver".to_string())),
            }),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![drv, m]
}

/// Drives the bitlines with the given data
pub fn bitline_driver(params: BitlineDriverParams) -> Module {
    let length = params.length;

    let vss = signal("vss");
    let din = signal("din");
    let din_b = signal("din_b");
    let bl = signal("bl");
    let br = signal("br");
    let we = signal("we");

    let ports = vec![
        port_inout(&vss),
        port_inout(&din),
        port_inout(&din_b),
        port_inout(&bl),
        port_inout(&br),
        port_input(&we),
    ];

    let mut m = Module {
        name: "bitline_driver".to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "bl_driver".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&bl),
            source: sig_conn(&din),
            gate: sig_conn(&we),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "br_driver".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&br),
            source: sig_conn(&din_b),
            gate: sig_conn(&we),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m
}

#[cfg(test)]
mod tests {
    use vlsir::circuit::Package;

    use crate::{save_bin, tech::all_external_modules};

    use super::*;

    #[test]
    fn test_netlist_bitline_driver() -> Result<(), Box<dyn std::error::Error>> {
        let drv = bitline_driver(BitlineDriverParams {
            length: 150,
            width: 2_000,
        });
        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "sramgen_bitline_driver".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![drv],
            ext_modules,
        };

        save_bin("bitline_driver", pkg)?;

        Ok(())
    }

    #[test]
    fn test_netlist_bitline_driver_array() -> Result<(), Box<dyn std::error::Error>> {
        let modules = bitline_driver_array(BitlineDriverArrayParams {
            name: "bitline_driver_array".to_string(),
            width: 64,
            instance_params: BitlineDriverParams {
                length: 150,
                width: 2_000,
            },
        });
        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "sramgen_bitline_driver_array".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules,
            ext_modules,
        };

        save_bin("bitline_driver_array", pkg)?;

        Ok(())
    }
}
