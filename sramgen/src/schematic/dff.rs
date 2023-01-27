use crate::config::dff::DffGridParams;
use crate::schematic::vlsir_api::{bus, signal, Instance, Module};
use crate::tech::openram_dff_ref;

pub fn dff_grid(params: &DffGridParams) -> Vec<Module> {
    let width = params.rows * params.cols;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");

    let d = bus("d", width);
    let q = bus("q", width);
    let q_b = bus("q_b", width);

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &vss]);
    m.add_ports_input(&[&clk, &d]);
    m.add_ports_output(&[&q, &q_b]);

    for i in 0..width {
        let mut inst = Instance::new(format!("dff_{i}"), openram_dff_ref());
        inst.add_conns(&[
            ("VDD", &vdd),
            ("GND", &vss),
            ("CLK", &clk),
            ("D", &d.get(i)),
            ("Q", &q.get(i)),
            ("Q_N", &q_b.get(i)),
        ]);

        m.add_instance(inst);
    }

    vec![m]
}
