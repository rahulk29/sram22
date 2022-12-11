use crate::config::gate::AndParams;
use crate::config::inv_chain::InvChainGridParams;
use crate::schematic::gate::and2;
use crate::schematic::inv_chain::inv_chain_grid;
use crate::schematic::vlsir_api::{local_reference, signal, Instance, Module};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct EdgeDetectorParams {
    pub name: String,
    pub num_inverters: usize,
    pub and_params: AndParams,
}

pub fn edge_detector(params: &EdgeDetectorParams) -> Vec<Module> {
    let num_inverters = params.num_inverters;
    assert_eq!(num_inverters % 2, 1);

    let EdgeDetectorParams {
        name, and_params, ..
    } = params;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = signal("din");
    let dout = signal("dout");
    let delayed = signal("delayed");

    let mut m = Module::new(name);
    m.add_port_input(&din);
    m.add_port_output(&dout);
    m.add_ports_inout(&[&vdd, &vss]);

    let inv_chain_name = format!("{}_invs", name);
    let chain = inv_chain_grid(&InvChainGridParams {
        name: inv_chain_name.clone(),
        rows: 1,
        cols: num_inverters,
    });
    let mut and2 = and2(and_params);

    let mut inst = Instance::new("delay_chain", local_reference(&inv_chain_name));
    inst.add_conns(&[
        ("din", &din),
        ("dout", &delayed),
        ("vdd", &vdd),
        ("vss", &vss),
    ]);

    m.add_instance(inst);

    let mut inst = Instance::new("and", local_reference(&and_params.name));
    inst.add_conns(&[
        ("a", &din),
        ("b", &delayed),
        ("y", &dout),
        ("vdd", &vdd),
        ("vss", &vss),
    ]);

    m.add_instance(inst);

    let mut modules = Vec::new();
    modules.push(chain);
    modules.append(&mut and2);
    modules.push(m);

    modules
}
