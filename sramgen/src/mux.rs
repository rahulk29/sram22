use std::collections::HashMap;

use pdkprims::{config::Int, mos::MosType};

use vlsir::{circuit::Module, reference::To, Reference};

use crate::{
    mos::Mosfet,
    utils::{bus, conns::conn_slice, port_inout, port_input, sig_conn, signal},
};

pub struct ColumnMuxParams {
    pub length: Int,
    pub width: Int,
}

pub struct ColumnMuxArrayParams {
    pub name: String,
    pub width: i64,
    pub instance_params: ColumnMuxParams,
}

pub fn column_mux_4_array(params: ColumnMuxArrayParams) -> Vec<Module> {
    assert!(params.width > 0);
    assert_eq!(params.width % 4, 0);

    let mux = column_mux_4(params.instance_params);

    let vdd = signal("vdd");
    let bl = bus("bl", params.width);
    let br = bus("br", params.width);
    let bl_out = bus("bl_out", params.width / 4);
    let br_out = bus("br_out", params.width / 4);
    let sel = bus("sel", 2);
    let sel_b = bus("sel_b", 2);

    let ports = vec![
        port_inout(&vdd),
        port_inout(&bl),
        port_inout(&br),
        port_inout(&bl_out),
        port_inout(&br_out),
        port_input(&sel),
        port_input(&sel_b),
    ];

    let mut m = Module {
        name: params.name.clone(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in (0..params.width).step_by(4) {
        let mut connections = HashMap::new();
        connections.insert("vdd".to_string(), sig_conn(&vdd));
        connections.insert("din".to_string(), conn_slice("bl", i + 3, i));
        connections.insert("sel".to_string(), sig_conn(&sel));
        connections.insert("sel_b".to_string(), sig_conn(&sel_b));
        connections.insert("dout".to_string(), conn_slice("bl_out", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_bl_{}", i),
            module: Some(Reference {
                to: Some(To::Local("column_mux_4".to_string())),
            }),
            parameters: HashMap::new(),
            connections,
        });

        let mut connections = HashMap::new();
        connections.insert("vdd".to_string(), sig_conn(&vdd));
        connections.insert("din".to_string(), conn_slice("br", i + 3, i));
        connections.insert("sel".to_string(), sig_conn(&sel));
        connections.insert("sel_b".to_string(), sig_conn(&sel_b));
        connections.insert("dout".to_string(), conn_slice("br_out", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_br_{}", i),
            module: Some(Reference {
                to: Some(To::Local("column_mux_4".to_string())),
            }),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![mux, m]
}

/// A 4 to 1 mux using PMOS devices
pub fn column_mux_4(params: ColumnMuxParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let din = bus("din", 4);
    let sel = bus("sel", 2);
    let sel_b = bus("sel_b", 2);
    let dout = signal("dout");

    let int_0 = signal("int_0");
    let int_1 = signal("int_1");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&din),
        port_inout(&dout),
        port_input(&sel),
        port_input(&sel_b),
    ];

    let mut m = Module {
        name: "column_mux_4".to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "s00".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&int_0),
            source: conn_slice("din", 0, 0),
            gate: conn_slice("sel", 0, 0),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "s01".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&int_0),
            source: conn_slice("din", 1, 1),
            gate: conn_slice("sel_b", 0, 0),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "s02".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&int_1),
            source: conn_slice("din", 2, 2),
            gate: conn_slice("sel", 0, 0),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "s03".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&int_1),
            source: conn_slice("din", 3, 3),
            gate: conn_slice("sel_b", 0, 0),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "s10".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&dout),
            source: sig_conn(&int_0),
            gate: conn_slice("sel", 1, 1),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "s11".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&dout),
            source: sig_conn(&int_1),
            gate: conn_slice("sel_b", 1, 1),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}

#[cfg(test)]
mod tests {
    use vlsir::circuit::Package;

    use crate::{save_bin, tech::all_external_modules, utils::save_modules};

    use super::*;

    #[test]
    fn test_netlist_column_mux_4() -> Result<(), Box<dyn std::error::Error>> {
        let mux = column_mux_4(ColumnMuxParams {
            length: 150,
            width: 2_000,
        });
        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "sramgen_column_mux_4".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![mux],
            ext_modules,
        };

        save_bin("column_mux_4", pkg)?;

        Ok(())
    }

    #[test]
    fn test_netlist_column_mux_4_array() -> Result<(), Box<dyn std::error::Error>> {
        let modules = column_mux_4_array(ColumnMuxArrayParams {
            name: "column_mux_4_array".to_string(),
            width: 64,
            instance_params: ColumnMuxParams {
                length: 150,
                width: 2_000,
            },
        });
        save_modules("column_mux_4_array", modules)?;
        Ok(())
    }
}
