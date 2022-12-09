use crate::config::gate::{AndParams, GateParams};
use crate::config::wl_driver::{WordlineDriverArrayParams, WordlineDriverParams};
use crate::schematic::gate::and2;
use crate::schematic::vlsir_api::{bus, local_reference, signal, Instance, Module};

pub fn wordline_driver_array(params: &WordlineDriverArrayParams) -> Vec<Module> {
    assert_eq!(params.width % 4, 0);

    let iparams = params.instance_params.clone();
    let mut wl_driver = wordline_driver(iparams);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = bus("din", params.width);
    let wl_en = signal("wl_en");
    let wl = bus("wl", params.width);

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &vss]);
    m.add_ports_input(&[&din, &wl_en]);
    m.add_port_output(&wl);

    for i in 0..params.width {
        let mut inst = Instance::new(
            format!("wl_driver_{}", i),
            local_reference(&params.instance_params.name),
        );
        inst.add_conns(&[
            ("VDD", &vdd),
            ("VSS", &vss),
            ("DIN", &din.get(i)),
            ("WL_EN", &wl_en),
            ("WL", &wl.get(i)),
        ]);
        m.add_instance(inst);
    }

    let mut modules = Vec::new();
    modules.append(&mut wl_driver);
    modules.push(m);
    modules
}

/// Drives the wordlines
pub fn wordline_driver(params: WordlineDriverParams) -> Vec<Module> {
    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let wl_en = signal("wl_en");
    let wl = signal("wl");

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &vss]);
    m.add_ports_input(&[&din, &wl_en]);
    m.add_port_output(&wl);

    let and2_name = format!("{}_and2", &params.name);
    let mut and2 = and2(&AndParams {
        name: and2_name.clone(),
        inv: GateParams {
            name: format!("{}_inv", &and2_name),
            size: params.inv_size,
            length: params.length,
        },
        nand: GateParams {
            name: format!("{}_nand", &and2_name),
            size: params.nand_size,
            length: params.length,
        },
    });

    let mut inst = Instance::new("and2", local_reference(and2_name));
    inst.add_conns(&[
        ("A", &din),
        ("B", &wl_en),
        ("Y", &wl),
        ("VDD", &vdd),
        ("VSS", &vss),
    ]);

    m.add_instance(inst);

    let mut modules = Vec::new();
    modules.append(&mut and2);
    modules.push(m);
    modules
}
