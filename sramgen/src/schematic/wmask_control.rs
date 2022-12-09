use std::collections::HashMap;

use vlsir::reference::To;
use vlsir::Reference;

use crate::config::wmask_control::WriteMaskControlParams;
use crate::schematic::gate::and2;
use crate::schematic::vlsir_api::{bus, signal, Instance, Module};

pub fn write_mask_control(params: &WriteMaskControlParams) -> Vec<Module> {
    let mut and = and2(&params.and_params);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let wr_en = signal("wr_en");
    let sel = bus("sel", params.width);
    let write_driver_en = bus("write_driver_en", params.width);

    let ports = vec![
        port_input(&wr_en),
        port_input(&sel),
        port_output(&write_driver_en),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let mut m = Module::new(&params.name);
    m.add_ports_input(&[&wr_en, &sel]);
    m.add_port_output(&write_driver_en);
    m.add_ports_inout(&[&vdd, &vss]);

    for i in 0..params.width {
        let mut inst = Instance::new(
            format!("and2_{}", i),
            local_reference(&params.and_params.name),
        );
        inst.add_conns(&[
            ("vdd", &vdd),
            ("vss", &vss),
            ("a", &sel.get(i)),
            ("b", &wr_en),
            ("y", &write_driver_en.get(i)),
        ]);
        m.add_instance(inst);
    }

    let mut modules = Vec::new();
    modules.append(&mut and);
    modules.push(m);
    modules
}
