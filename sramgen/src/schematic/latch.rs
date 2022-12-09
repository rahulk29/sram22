use std::collections::HashMap;

use crate::config::gate::GateParams;
use crate::schematic::gate::nor2;
use crate::schematic::vlsir_api::{local_reference, signal, Instance, Module};

pub struct SrLatchParams {
    pub name: String,
    pub nor: GateParams,
}

pub fn sr_latch(params: &SrLatchParams) -> Vec<Module> {
    let SrLatchParams {
        name,
        nor: nor_params,
    } = params;
    let vdd = signal("vdd");
    let vss = signal("vss");
    let s = signal("s");
    let r = signal("r");
    let q = signal("q");
    let qb = signal("qb");

    let nor = nor2(nor_params);

    let mut m = Module::new(name);
    m.add_ports_input(&[&s, &r]);
    m.add_ports_output(&[&q, &qb]);
    m.add_ports_inout(&[&vdd, &vss]);

    let mut inst = Instance::new("nor_set", local_reference(&params.nor.name));
    inst.add_conns(&[
        ("A", &s),
        ("B", &q),
        ("Y", &qb),
        ("VDD", &vdd),
        ("GND", &vss),
    ]);
    m.add_instance(inst);

    let mut inst = Instance::new("nor_reset", local_reference(&params.nor.name));
    inst.add_conns(&[
        ("A", &r),
        ("B", &qb),
        ("Y", &q),
        ("VDD", &vdd),
        ("GND", &vss),
    ]);
    m.add_instance(inst);

    vec![nor, m]
}
