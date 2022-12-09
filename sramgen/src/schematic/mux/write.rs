use pdkprims::mos::MosType;

use crate::config::mux::{WriteMuxArrayParams, WriteMuxParams};
use crate::schematic::mos::Mosfet;
use crate::schematic::vlsir_api::{bus, local_reference, signal, Instance, Module};

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

    let mux = column_write_mux(mux_params);

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

    let mut m = Module::new(name);
    m.add_port_input(&we);
    if enable_wmask {
        m.add_port_input(&wmask);
    }
    m.add_ports_input(&[&data, &data_b]);
    m.add_ports_inout(&[&bl, &br, &vss]);

    for i in 0..cols {
        let sel_idx = i % mux_ratio;
        let group_idx = i / mux_ratio;
        let wmask_idx = i / bpmask;
        let mut inst = Instance::new(format!("mux_{}", i), local_reference(&mux_params.name));
        inst.add_conns(&[
            ("WE", &we.get(sel_idx)),
            ("DATA", &data.get(group_idx)),
            ("DATA_B", &data_b.get(group_idx)),
            ("BL", &bl.get(i)),
            ("BR", &br.get(i)),
            ("VSS", &vss),
        ]);
        if enable_wmask {
            inst.add_conns(&[("wmask", &wmask.get(wmask_idx))]);
        }
        m.add_instance(inst);
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

    let mut m = Module::new(name);
    m.add_port_input(&we);
    if params.wmask {
        m.add_port_input(&wmask);
    }
    m.add_ports_input(&[&data, &data_b]);
    m.add_ports_inout(&[&bl, &br, &vss]);

    m.add_instance(
        Mosfet {
            name: "MMUXBR".to_string(),
            width: params.width,
            length,
            drain: br.clone(),
            source: x.clone(),
            gate: data.clone(),
            body: vss.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m.add_instance(
        Mosfet {
            name: "MMUXBL".to_string(),
            width: params.width,
            length,
            drain: bl.clone(),
            source: x.clone(),
            gate: data_b.clone(),
            body: vss.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    if params.wmask {
        m.add_instance(
            Mosfet {
                name: "MWMASK".to_string(),
                width: params.width,
                length,
                drain: x.clone(),
                source: y.clone(),
                gate: wmask.clone(),
                body: vss.clone(),
                mos_type: MosType::Nmos,
            }
            .into(),
        );
    }

    m.add_instance(
        Mosfet {
            name: "MPD".to_string(),
            width: params.width,
            length,
            drain: if params.wmask { y.clone() } else { x.clone() },
            source: vss.clone(),
            gate: we.clone(),
            body: vss.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m
}
