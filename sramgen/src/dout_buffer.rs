use std::collections::HashMap;

use pdkprims::config::Int;
use pdkprims::mos::MosType;

use vlsir::circuit::Module;
use vlsir::reference::To;
use vlsir::Reference;

use crate::mos::Mosfet;
use crate::utils::conns::conn_slice;
use crate::utils::{bus, port_inout, port_input, port_output, sig_conn, signal};

pub struct DoutBufParams {
    pub length: Int,
    pub nw1: Int,
    pub pw1: Int,
    pub nw2: Int,
    pub pw2: Int,
}

pub struct DoutBufArrayParams {
    pub name: String,
    pub width: i64,
    pub instance_params: DoutBufParams,
}

pub fn dout_buf_array(params: DoutBufArrayParams) -> Vec<Module> {
    assert!(params.width > 0);

    let inv = dout_buf(params.instance_params);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din1 = bus("din1", params.width);
    let din2 = bus("din2", params.width);
    let dout1 = bus("dout1", params.width);
    let dout2 = bus("dout2", params.width);

    let ports = vec![
        port_input(&din1),
        port_input(&din2),
        port_output(&dout1),
        port_output(&dout2),
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
        let mut connections = HashMap::new();
        connections.insert("vdd".to_string(), sig_conn(&vdd));
        connections.insert("vss".to_string(), sig_conn(&vss));
        connections.insert("din1".to_string(), conn_slice("din1", i, i));
        connections.insert("din2".to_string(), conn_slice("din2", i, i));
        connections.insert("dout1".to_string(), conn_slice("dout1", i, i));
        connections.insert("dout2".to_string(), conn_slice("dout2", i, i));
        m.instances.push(vlsir::circuit::Instance {
            name: format!("buf_{}", i),
            module: Some(Reference {
                to: Some(To::Local("dout_buf".to_string())),
            }),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![inv, m]
}

pub fn dout_buf(params: DoutBufParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din1 = signal("din1");
    let din2 = signal("din2");
    let dout1 = signal("dout1");
    let dout2 = signal("dout2");
    let x1 = signal("x1");
    let x2 = signal("x2");

    let ports = vec![
        port_input(&din1),
        port_input(&din2),
        port_output(&dout1),
        port_output(&dout2),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let mut m = Module {
        name: "dout_buf".to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for (din, x, dout, suffix) in [(&din1, &x1, &dout1, "1"), (&din2, &x2, &dout2, "2")] {
        m.instances.push(
            Mosfet {
                name: format!("MP1{}", suffix),
                width: params.pw1,
                length,
                drain: sig_conn(x),
                source: sig_conn(&vdd),
                gate: sig_conn(din),
                body: sig_conn(&vdd),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m.instances.push(
            Mosfet {
                name: format!("MN1{}", suffix),
                width: params.nw1,
                length,
                drain: sig_conn(x),
                source: sig_conn(&vss),
                gate: sig_conn(din),
                body: sig_conn(&vss),
                mos_type: MosType::Nmos,
            }
            .into(),
        );
        m.instances.push(
            Mosfet {
                name: format!("MP2{}", suffix),
                width: params.pw2,
                length,
                drain: sig_conn(dout),
                source: sig_conn(&vdd),
                gate: sig_conn(x),
                body: sig_conn(&vdd),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m.instances.push(
            Mosfet {
                name: format!("MN2{}", suffix),
                width: params.nw2,
                length,
                drain: sig_conn(dout),
                source: sig_conn(&vss),
                gate: sig_conn(x),
                body: sig_conn(&vss),
                mos_type: MosType::Nmos,
            }
            .into(),
        );
    }

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
    fn test_netlist_dout_buf() -> Result<(), Box<dyn std::error::Error>> {
        let buf = dout_buf(DoutBufParams {
            length: 150,
            nw1: 1_000,
            pw1: 1_600,
            nw2: 2_000,
            pw2: 3_200,
        });
        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "dout_buf".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![buf],
            ext_modules,
        };

        save_bin("dout_buf", pkg)?;

        Ok(())
    }

    #[test]
    fn test_netlist_dout_buf_array() -> Result<(), Box<dyn std::error::Error>> {
        let modules = dout_buf_array(DoutBufArrayParams {
            name: "dout_buf_array".to_string(),
            width: 16,
            instance_params: DoutBufParams {
                length: 150,
                nw1: 1_000,
                pw1: 1_600,
                nw2: 2_000,
                pw2: 3_200,
            },
        });
        save_modules("dout_buf_array", modules)?;
        Ok(())
    }
}
