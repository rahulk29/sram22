use pdkprims::mos::MosType;

use crate::config::precharge::{PrechargeArrayParams, PrechargeParams};
use crate::schematic::mos::Mosfet;
use crate::schematic::vlsir_api::{bus, local_reference, signal, Instance, Module};

pub fn precharge_array(params: &PrechargeArrayParams) -> Vec<Module> {
    assert!(params.width > 0);

    let pc = precharge(&params.instance_params);

    let vdd = signal("vdd");
    let en_b = signal("en_b");
    let bl = bus("bl", params.width);
    let br = bus("br", params.width);

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &bl, &br]);
    m.add_port_input(&en_b);

    for i in 0..params.width {
        let mut inst = Instance::new(
            format!("precharge_{}", i),
            local_reference(&params.instance_params.name),
        );
        inst.add_conns(&[
            ("VDD", &vdd),
            ("EN_B", &en_b),
            ("BL", &bl.get(i)),
            ("BR", &br.get(i)),
        ]);
        m.add_instance(inst);
    }

    vec![pc, m]
}

pub fn precharge(params: &PrechargeParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let bl = signal("bl");
    let br = signal("br");
    let en_b = signal("en_b");

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &bl, &br]);
    m.add_port_input(&en_b);

    m.add_instance(
        Mosfet {
            name: "bl_pull_up".to_string(),
            width: params.pull_up_width,
            length,
            drain: bl.clone(),
            source: vdd.clone(),
            gate: en_b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "br_pull_up".to_string(),
            width: params.pull_up_width,
            length,
            drain: br.clone(),
            source: vdd.clone(),
            gate: en_b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.add_instance(
        Mosfet {
            name: "equalizer".to_string(),
            width: params.equalizer_width,
            length,
            drain: bl.clone(),
            source: br.clone(),
            gate: en_b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}
