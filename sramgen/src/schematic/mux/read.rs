use std::collections::HashMap;

use pdkprims::mos::MosType;

use crate::config::mux::{ReadMuxArrayParams, ReadMuxParams};
use crate::schematic::mos::Mosfet;
use crate::schematic::vlsir_api::{bus, local_reference, signal, Instance, Module};

pub fn read_mux_array(params: &ReadMuxArrayParams) -> Vec<Module> {
    let &ReadMuxArrayParams {
        cols, mux_ratio, ..
    } = params;
    let ReadMuxArrayParams {
        name, mux_params, ..
    } = params;

    let mux = read_mux(mux_params);
    assert_eq!(mux_ratio % 2, 0);
    assert_eq!(cols % mux_ratio, 0);

    let sel_b = bus("sel_b", mux_ratio);
    let bl = bus("bl", cols);
    let br = bus("br", cols);
    let bl_out = bus("bl_out", cols / mux_ratio);
    let br_out = bus("br_out", cols / mux_ratio);
    let vdd = signal("vdd");

    let mut m = Module::new(name);
    m.add_port_input(&sel_b);
    m.add_ports_inout(&[&bl, &br, &bl_out, &br_out, &vdd]);

    for i in 0..cols {
        let output_idx = i / mux_ratio;
        let sel_idx = i % mux_ratio;
        let mut inst = Instance::new(format!("mux_{}", i), local_reference(&mux_params.name));
        inst.add_conns(&[
            ("VDD", &vdd),
            ("BL", &bl.get(i)),
            ("BR", &br.get(i)),
            ("BL_OUT", &bl_out.get(output_idx)),
            ("BR_OUT", &br_out.get(output_idx)),
            ("SEL_B", &sel_b.get(sel_idx)),
        ]);
        m.add_instance(inst);
    }

    vec![mux, m]
}

/// A read mux using PMOS devices
pub fn read_mux(params: &ReadMuxParams) -> Module {
    let length = params.length;

    let sel_b = signal("sel_b");
    let bl = signal("bl");
    let br = signal("br");
    let bl_out = signal("bl_out");
    let br_out = signal("br_out");
    let vdd = signal("vdd");

    let mut m = Module::new(&params.name);
    m.add_port_input(&sel_b);
    m.add_ports_inout(&[&bl, &br, &bl_out, &br_out, &vdd]);

    m.instances.push(
        Mosfet {
            name: "MBL".to_string(),
            width: params.width,
            length,
            source: bl.clone(),
            drain: bl_out.clone(),
            gate: sel_b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.instances.push(
        Mosfet {
            name: "MBR".to_string(),
            width: params.width,
            length,
            source: br.clone(),
            drain: br_out.clone(),
            gate: sel_b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}
