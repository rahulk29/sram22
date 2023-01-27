use pdkprims::mos::MosType;

use crate::config::col_inv::{ColInvArrayParams, ColInvParams};
use crate::schematic::mos::Mosfet;
use crate::schematic::vlsir_api::{bus, local_reference, signal, Instance, Module};

pub fn col_inv_array(params: &ColInvArrayParams) -> Vec<Module> {
    assert!(params.width > 0);

    let inv = col_inv(&params.instance_params);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = bus("din", params.width);
    let din_b = bus("din_b", params.width);

    let mut m = Module::new(&params.name);
    m.add_port_input(&din);
    m.add_port_output(&din_b);
    m.add_ports_inout(&[&vdd, &vss]);

    for i in 0..params.width {
        let mut inst = Instance::new(format!("inv_{i}"), local_reference("col_data_inv"));
        inst.add_conns(&[
            ("vdd", &vdd),
            ("vss", &vss),
            ("din", &din.get(i)),
            ("din_b", &din_b.get(i)),
        ]);
        m.add_instance(inst);
    }

    vec![inv, m]
}

pub fn col_inv(params: &ColInvParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let din_b = signal("din_b");

    let mut m = Module::new("col_data_inv");
    m.add_port_input(&din);
    m.add_port_output(&din_b);
    m.add_ports_inout(&[&vdd, &vss]);

    m.add_instance(
        Mosfet {
            name: "MP0".to_string(),
            width: params.pwidth,
            length,
            drain: din_b.clone(),
            source: vdd.clone(),
            gate: din.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m.add_instance(
        Mosfet {
            name: "MN0".to_string(),
            width: params.nwidth,
            length,
            drain: din_b,
            source: vss.clone(),
            gate: din,
            body: vss,
            mos_type: MosType::Nmos,
        }
        .into(),
    );

    m
}
