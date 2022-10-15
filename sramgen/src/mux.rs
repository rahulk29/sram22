use std::collections::HashMap;

use pdkprims::config::Int;
use pdkprims::mos::MosType;

use vlsir::circuit::Module;
use vlsir::reference::To;
use vlsir::Reference;

use crate::mos::Mosfet;
use crate::utils::conns::conn_slice;
use crate::utils::{bus, conn_map, port_inout, port_input, sig_conn, signal};

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

    for i in 0..(params.width / 4) {
        let mut connections = HashMap::new();
        connections.insert("vdd".to_string(), sig_conn(&vdd));
        connections.insert("din".to_string(), conn_slice("bl", 4 * i + 3, 4 * i));
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
        connections.insert("din".to_string(), conn_slice("br", 4 * i + 3, 4 * i));
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

pub fn column_read_mux_2_array(params: ColumnMuxArrayParams) -> Vec<Module> {
    let mux = column_read_mux_2(params.instance_params);

    let reference = Reference {
        to: Some(To::Local("column_read_mux_2".to_string())),
    };
    vec![
        mux,
        mux_2_array_inner(params.width, params.name, &reference, true),
    ]
}

pub fn column_write_mux_2_array(params: ColumnMuxArrayParams) -> Vec<Module> {
    let mux = column_write_mux_2(params.instance_params);

    let reference = Reference {
        to: Some(To::Local("column_write_mux_2".to_string())),
    };
    vec![
        mux,
        write_mux_2_array_inner(params.width, params.name, &reference),
    ]
}

fn write_mux_2_array_inner(width: i64, name: String, reference: &Reference) -> Module {
    assert!(width > 0);
    assert_eq!(width % 2, 0);

    let vss = signal("vss");
    let bl = bus("bl", width);
    let br = bus("br", width);
    let data = bus("data", width / 2);
    let data_b = bus("data_b", width / 2);
    let we0 = signal("we_0_0");
    let we1 = signal("we_1_0");

    let ports = vec![
        port_input(&we0),
        port_input(&we1),
        port_inout(&bl),
        port_inout(&br),
        port_input(&data),
        port_input(&data_b),
        port_inout(&vss),
    ];

    let mut m = Module {
        name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..(width / 2) {
        let mut connections = HashMap::new();
        connections.insert("en", sig_conn(&we0));
        connections.insert("data", conn_slice("data", i, i));
        connections.insert("data_b", conn_slice("data_b", i, i));
        connections.insert("bl", conn_slice("bl", 2 * i, 2 * i));
        connections.insert("br", conn_slice("br", 2 * i, 2 * i));
        connections.insert("vss", sig_conn(&vss));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_0_{}", i),
            module: Some(reference.clone()),
            parameters: HashMap::new(),
            connections: conn_map(connections),
        });

        let mut connections = HashMap::new();
        connections.insert("en", sig_conn(&we1));
        connections.insert("data", conn_slice("data", i, i));
        connections.insert("data_b", conn_slice("data_b", i, i));
        connections.insert("bl", conn_slice("bl", 2 * i + 1, 2 * i + 1));
        connections.insert("br", conn_slice("br", 2 * i + 1, 2 * i + 1));
        connections.insert("vss", sig_conn(&vss));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_1_{}", i),
            module: Some(reference.clone()),
            parameters: HashMap::new(),
            connections: conn_map(connections),
        });
    }

    m
}

fn mux_2_array_inner(
    width: i64,
    name: impl Into<String>,
    reference: &Reference,
    vdd: bool,
) -> Module {
    assert!(width > 0);
    assert_eq!(width % 2, 0);

    let (pwr_name, pwr_sig) = if vdd {
        ("vdd", signal("vdd"))
    } else {
        ("vss", signal("vss"))
    };

    let bl = bus("bl", width);
    let br = bus("br", width);
    let bl_out = bus("bl_out", width / 2);
    let br_out = bus("br_out", width / 2);
    let sel = bus("sel", 2);

    let ports = vec![
        port_inout(&pwr_sig),
        port_inout(&bl),
        port_inout(&br),
        port_inout(&bl_out),
        port_inout(&br_out),
        port_input(&sel),
    ];

    let mut m = Module {
        name: name.into(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..(width / 2) {
        let mut connections = HashMap::new();
        connections.insert(pwr_name, sig_conn(&pwr_sig));
        connections.insert("din", conn_slice("bl", 2 * i + 1, 2 * i));
        connections.insert("sel", sig_conn(&sel));
        connections.insert("dout", conn_slice("bl_out", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_bl_{}", i),
            module: Some(reference.clone()),
            parameters: HashMap::new(),
            connections: conn_map(connections),
        });

        let mut connections = HashMap::new();
        connections.insert(pwr_name, sig_conn(&pwr_sig));
        connections.insert("din", conn_slice("br", 2 * i + 1, 2 * i));
        connections.insert("sel", sig_conn(&sel));
        connections.insert("dout", conn_slice("br_out", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_br_{}", i),
            module: Some(reference.clone()),
            parameters: HashMap::new(),
            connections: conn_map(connections),
        });
    }

    m
}

/// A 2 to 1 mux using PMOS devices
pub fn column_read_mux_2(params: ColumnMuxParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let din = bus("din", 2);
    let sel = bus("sel", 2);
    let dout = signal("dout");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&din),
        port_inout(&dout),
        port_input(&sel),
    ];

    let mut m = Module {
        name: "column_read_mux_2".to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "MP0".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&dout),
            source: conn_slice("din", 0, 0),
            gate: conn_slice("sel", 0, 0),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "MP1".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&dout),
            source: conn_slice("din", 1, 1),
            gate: conn_slice("sel", 1, 1),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}

/// A 2 to 1 mux using NMOS devices
pub fn column_write_mux_2(params: ColumnMuxParams) -> Module {
    let length = params.length;

    let en = signal("en");
    let data = signal("data");
    let data_b = signal("data_b");
    let bl = signal("bl");
    let br = signal("br");
    let vss = signal("vss");
    let int = signal("int");

    let ports = vec![
        port_input(&en),
        port_input(&data),
        port_input(&data_b),
        port_inout(&bl),
        port_inout(&br),
        port_inout(&vss),
    ];

    let mut m = Module {
        name: "column_write_mux_2".to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "MN0".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&br),
            source: sig_conn(&int),
            gate: sig_conn(&data),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "MN1".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&bl),
            source: sig_conn(&int),
            gate: sig_conn(&data_b),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "MN2".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&int),
            source: sig_conn(&vss),
            gate: sig_conn(&en),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m
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

    use crate::save_bin;
    use crate::tech::all_external_modules;
    use crate::utils::save_modules;

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

    #[test]
    fn test_column_write_mux_2_array() -> Result<(), Box<dyn std::error::Error>> {
        let modules = column_write_mux_2_array(ColumnMuxArrayParams {
            name: "column_write_mux_2_array".to_string(),
            width: 32,
            instance_params: ColumnMuxParams {
                length: 150,
                width: 2_000,
            },
        });
        save_modules("column_write_mux_2_array", modules)?;
        Ok(())
    }

    #[test]
    fn test_column_read_mux_2_array() -> Result<(), Box<dyn std::error::Error>> {
        let modules = column_read_mux_2_array(ColumnMuxArrayParams {
            name: "column_read_mux_2_array".to_string(),
            width: 64,
            instance_params: ColumnMuxParams {
                length: 150,
                width: 1_200,
            },
        });
        save_modules("column_read_mux_2_array", modules)?;
        Ok(())
    }
}
