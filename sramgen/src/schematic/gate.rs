use pdkprims::mos::MosType;
use serde::{Deserialize, Serialize};

use crate::config::gate::{AndParams, GateParams, Size};
use crate::schematic::mos::Mosfet;
use crate::schematic::vlsir_api::{local_reference, signal, Instance, Module};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct Gate {
    pub gate_type: GateType,
    pub size: Size,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum GateType {
    Inv,
    Nand2,
    Nand3,
}

impl GateType {
    pub fn num_inputs(&self) -> usize {
        match *self {
            GateType::Inv => 1,
            GateType::Nand2 => 2,
            GateType::Nand3 => 3,
        }
    }
}

impl Gate {
    #[inline]
    pub fn num_inputs(&self) -> usize {
        self.gate_type.num_inputs()
    }

    #[inline]
    pub fn new(gate_type: GateType, size: Size) -> Self {
        Self { gate_type, size }
    }
}

impl From<GateType> for fanout::GateType {
    fn from(x: GateType) -> Self {
        match x {
            GateType::Inv => fanout::GateType::INV,
            GateType::Nand2 => fanout::GateType::NAND2,
            GateType::Nand3 => fanout::GateType::NAND3,
        }
    }
}

pub fn and2(params: &AndParams) -> Vec<Module> {
    let vdd = signal("vdd");
    let a = signal("a");
    let b = signal("b");
    let y = signal("y");
    let vss = signal("vss");

    let mut m = Module::new(&params.name);
    m.add_ports_input(&[&a, &b]);
    m.add_port_output(&y);
    m.add_ports_inout(&[&vdd, &vss]);

    let nand_name = format!("{}_nand", &params.name);
    let nand = nand2(&GateParams {
        name: nand_name.clone(),
        size: params.nand.size,
        length: params.nand.length,
    });
    let inv_name = format!("{}_inv", &params.name);
    let inv = inv(&GateParams {
        name: inv_name.clone(),
        size: params.inv.size,
        length: params.inv.length,
    });

    let tmp = signal("tmp");

    // nand
    let mut inst = Instance::new("nand", local_reference(nand_name));
    inst.add_conns(&[
        ("VDD", &vdd),
        ("GND", &vss),
        ("A", &a),
        ("B", &b),
        ("Y", &tmp),
    ]);

    m.add_instance(inst);

    // inv
    let mut inst = Instance::new("inv", local_reference(inv_name));
    inst.add_conns(&[("VDD", &vdd), ("GND", &vss), ("DIN", &tmp), ("DIN_B", &y)]);

    m.add_instance(inst);

    vec![nand, inv, m]
}

pub fn nand2(params: &GateParams) -> Module {
    let length = params.length;
    let size = params.size;

    let gnd = signal("gnd");
    let vdd = signal("vdd");
    let a = signal("a");
    let b = signal("b");
    let y = signal("y");
    let x = signal("x");

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&gnd, &vdd]);
    m.add_ports_input(&[&a, &b]);
    m.add_port_output(&y);

    m.add_instance(
        Mosfet {
            name: "n1".to_string(),
            width: size.nmos_width,
            length,
            drain: x.clone(),
            source: gnd.clone(),
            gate: a.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "n2".to_string(),
            width: size.nmos_width,
            length,
            drain: y.clone(),
            source: x.clone(),
            gate: b.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p1".to_string(),
            width: size.pmos_width,
            length,
            drain: y.clone(),
            source: vdd.clone(),
            gate: a.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p2".to_string(),
            width: size.pmos_width,
            length,
            drain: y.clone(),
            source: vdd.clone(),
            gate: b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}

pub fn nor2(params: &GateParams) -> Module {
    let length = params.length;
    let size = params.size;

    let gnd = signal("gnd");
    let vdd = signal("vdd");
    let a = signal("a");
    let b = signal("b");
    let y = signal("y");
    let x = signal("x");

    let mut m = Module::new(&params.name);
    m.add_ports_input(&[&a, &b]);
    m.add_port_output(&y);
    m.add_ports_inout(&[&vdd, &gnd]);

    m.add_instance(
        Mosfet {
            name: "n1".to_string(),
            width: size.nmos_width,
            length,
            drain: y.clone(),
            source: gnd.clone(),
            gate: a.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "n2".to_string(),
            width: size.nmos_width,
            length,
            drain: y.clone(),
            source: gnd.clone(),
            gate: b.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p1".to_string(),
            width: size.pmos_width,
            length,
            drain: y.clone(),
            source: x.clone(),
            gate: a.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p2".to_string(),
            width: size.pmos_width,
            length,
            drain: x.clone(),
            source: vdd.clone(),
            gate: b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}

pub fn nand3(params: &GateParams) -> Module {
    let length = params.length;
    let size = params.size;

    let gnd = signal("gnd");
    let vdd = signal("vdd");
    let a = signal("a");
    let b = signal("b");
    let c = signal("c");
    let y = signal("y");
    let x1 = signal("x1");
    let x2 = signal("x2");

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&gnd, &vdd]);
    m.add_ports_input(&[&a, &b, &c]);
    m.add_port_output(&y);

    m.add_instance(
        Mosfet {
            name: "n1".to_string(),
            width: size.nmos_width,
            length,
            drain: x1.clone(),
            source: gnd.clone(),
            gate: a.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "n2".to_string(),
            width: size.nmos_width,
            length,
            drain: x2.clone(),
            source: x1.clone(),
            gate: b.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "n3".to_string(),
            width: size.nmos_width,
            length,
            drain: y.clone(),
            source: x2.clone(),
            gate: c.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p1".to_string(),
            width: size.pmos_width,
            length,
            drain: y.clone(),
            source: vdd.clone(),
            gate: a.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p2".to_string(),
            width: size.pmos_width,
            length,
            drain: y.clone(),
            source: vdd.clone(),
            gate: b.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p3".to_string(),
            width: size.pmos_width,
            length,
            drain: y.clone(),
            source: vdd.clone(),
            gate: c.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}

pub fn inv(params: &GateParams) -> Module {
    let length = params.length;
    let size = params.size;

    let gnd = signal("gnd");
    let vdd = signal("vdd");
    let din = signal("din");
    let din_b = signal("din_b");

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&gnd, &vdd]);
    m.add_port_input(&din);
    m.add_port_output(&din_b);

    m.add_instance(
        Mosfet {
            name: "n".to_string(),
            width: size.nmos_width,
            length,
            drain: din_b.clone(),
            source: gnd.clone(),
            gate: din.clone(),
            body: gnd.clone(),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.add_instance(
        Mosfet {
            name: "p".to_string(),
            width: size.pmos_width,
            length,
            drain: din_b.clone(),
            source: vdd.clone(),
            gate: din.clone(),
            body: vdd.clone(),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}
