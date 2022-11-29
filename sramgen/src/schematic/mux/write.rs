use std::collections::HashMap;

use pdkprims::mos::MosType;
use vlsir::circuit::Module;

use crate::config::mux::{WriteMuxArrayParams, WriteMuxParams};
use crate::schematic::conns::{
    bus, conn_map, conn_slice, port_inout, port_input, sig_conn, signal,
};
use crate::schematic::local_reference;
use crate::schematic::mos::Mosfet;

pub fn write_mux_array(params: &WriteMuxArrayParams) -> Vec<Module> {
    let &WriteMuxArrayParams {
        cols,
        mux_ratio,
        wmask_width,
        ..
    } = params;
    let WriteMuxArrayParams {
        name, mux_params, ..
    } = params;

    let mux_ratio = mux_ratio as i64;
    let wmask_width = wmask_width as i64;
    let cols = cols as i64;

    let mux = column_write_mux(mux_params);

    assert!(cols > 0);
    assert_eq!(cols % 2, 0);
    assert_eq!(cols % (mux_ratio * wmask_width), 0);

    // bits per word
    let bpw = cols / mux_ratio;

    // bits per mask signal
    let bpmask = cols / wmask_width;

    let enable_wmask = wmask_width > 1;

    let vss = signal("vss");
    let bl = bus("bl", cols);
    let br = bus("br", cols);
    let wmask = bus("wmask", wmask_width);
    let data = bus("data", bpw);
    let data_b = bus("data_b", bpw);
    let we = bus("we", mux_ratio);

    let mut ports = vec![
        port_input(&we),
        port_inout(&data),
        port_inout(&data_b),
        port_input(&bl),
        port_input(&br),
        port_inout(&vss),
    ];

    if enable_wmask {
        ports.insert(1, port_input(&wmask));
    }

    let mut m = Module {
        name: name.to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..cols {
        let sel_idx = i % mux_ratio;
        let group_idx = i / mux_ratio;
        let wmask_idx = i / bpmask;
        let mut connections = HashMap::new();
        connections.insert("we", conn_slice("we", sel_idx, sel_idx));
        connections.insert("data", conn_slice("data", group_idx, group_idx));
        connections.insert("data_b", conn_slice("data_b", group_idx, group_idx));
        connections.insert("bl", conn_slice("bl", i, i));
        connections.insert("br", conn_slice("br", i, i));
        connections.insert("vss", sig_conn(&vss));
        if enable_wmask {
            connections.insert("wmask", conn_slice("wmask", wmask_idx, wmask_idx));
        }
        m.instances.push(vlsir::circuit::Instance {
            name: format!("mux_{}", i),
            module: local_reference(&mux_params.name),
            parameters: HashMap::new(),
            connections: conn_map(connections),
        });
    }

    vec![mux, m]
}

pub fn column_write_mux(params: &WriteMuxParams) -> Module {
    let name = &params.name;
    let length = params.length;

    let we = signal("we");
    let data = signal("data");
    let data_b = signal("data_b");
    let bl = signal("bl");
    let br = signal("br");
    let vss = signal("vss");
    let x = signal("x");
    let y = signal("y");
    let wmask = signal("wmask");

    let mut ports = vec![
        port_input(&we),
        port_input(&data),
        port_input(&data_b),
        port_inout(&bl),
        port_inout(&br),
        port_inout(&vss),
    ];

    if params.wmask {
        ports.insert(1, port_inout(&wmask));
    }

    let mut m = Module {
        name: name.to_string(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "MMUXBR".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&br),
            source: sig_conn(&x),
            gate: sig_conn(&data),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m.instances.push(
        Mosfet {
            name: "MMUXBL".to_string(),
            width: params.width,
            length,
            drain: sig_conn(&bl),
            source: sig_conn(&x),
            gate: sig_conn(&data_b),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    if params.wmask {
        m.instances.push(
            Mosfet {
                name: "MWMASK".to_string(),
                width: params.width,
                length,
                drain: sig_conn(&x),
                source: sig_conn(&y),
                gate: sig_conn(&wmask),
                body: sig_conn(&vss),
                mos_type: MosType::Nmos,
            }
            .into(),
        );
    }

    m.instances.push(
        Mosfet {
            name: "MPD".to_string(),
            width: params.width,
            length,
            drain: sig_conn(if params.wmask { &y } else { &x }),
            source: sig_conn(&vss),
            gate: sig_conn(&we),
            body: sig_conn(&vss),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m
}
