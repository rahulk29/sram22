use crate::config::inv_chain::InvChainGridParams;
use crate::schematic::vlsir_api::{bus, signal, Instance, Module};
use crate::tech::control_logic_inv_ref;

pub fn inv_chain_grid(params: &InvChainGridParams) -> Module {
    let &InvChainGridParams { rows, cols, .. } = params;
    let name = &params.name;
    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let dout = signal("dout");
    let int = bus("int", rows * cols - 1);

    let mut m = Module::new(name);
    m.add_port_input(&din);
    m.add_port_output(&dout);
    m.add_ports_inout(&[&vdd, &vss]);

    for i in 0..(rows * cols) {
        let input = if i == 0 { din.clone() } else { int.get(i - 1) };
        let output = if i == rows * cols - 1 {
            dout.clone()
        } else {
            int.get(i)
        };

        let mut inst = Instance::new(format!("inv_{}", i), control_logic_inv_ref());
        inst.add_conns(&[
            ("DIN", &input),
            ("DIN_B", &output),
            ("VDD", &vdd),
            ("VSS", &vss),
        ]);
        m.add_instance(inst);
    }

    m
}
