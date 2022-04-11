use pdkprims::{config::Int, mos::MosType};
use serde::{Deserialize, Serialize};
use vlsir::circuit::{connection::Stype, port, Connection, Module, Port, Signal};

use crate::{
    mos::Mosfet,
    utils::{sig_conn, signal},
};

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct Size {
    pub nmos_width: Int,
    pub pmos_width: Int,
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

pub struct GateParams {
    pub name: String,
    pub size: Size,
    pub length: Int,
}

pub fn nand2(params: GateParams) -> Module {
    let length = params.length;
    let size = params.size;

    let mut m = Module {
        name: params.name,
        ports: vec![],
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };
    let gnd = signal("gnd");
    let vdd = signal("vdd");
    let a = signal("a");
    let b = signal("b");
    let y = signal("y");
    let x = signal("x");

    for sig in [gnd.clone(), vdd.clone()] {
        m.ports.push(Port {
            signal: Some(sig),
            direction: port::Direction::Inout as i32,
        });
    }
    for sig in [a.clone(), b.clone()] {
        m.ports.push(Port {
            signal: Some(sig),
            direction: port::Direction::Input as i32,
        });
    }
    m.ports.push(Port {
        signal: Some(y.clone()),
        direction: port::Direction::Output as i32,
    });

    m.instances.push(
        Mosfet {
            name: "n1".to_string(),
            width: size.nmos_width,
            length,
            drain: sig_conn(&x),
            source: sig_conn(&gnd),
            gate: sig_conn(&a),
            body: sig_conn(&gnd),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.instances.push(
        Mosfet {
            name: "n2".to_string(),
            width: size.nmos_width,
            length,
            drain: sig_conn(&y),
            source: sig_conn(&x),
            gate: sig_conn(&b),
            body: sig_conn(&gnd),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.instances.push(
        Mosfet {
            name: "p1".to_string(),
            width: size.pmos_width,
            length,
            drain: sig_conn(&y),
            source: sig_conn(&vdd),
            gate: sig_conn(&a),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );
    m.instances.push(
        Mosfet {
            name: "p2".to_string(),
            width: size.pmos_width,
            length,
            drain: sig_conn(&y),
            source: sig_conn(&vdd),
            gate: sig_conn(&b),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}

pub fn inv(params: GateParams) -> Module {
    let length = params.length;
    let size = params.size;

    let gnd = signal("gnd");
    let vdd = signal("vdd");
    let din = signal("din");
    let dinb = signal("din_b");

    let ports = vec![
        Port {
            signal: Some(gnd.clone()),
            direction: port::Direction::Inout as i32,
        },
        Port {
            signal: Some(vdd.clone()),
            direction: port::Direction::Inout as i32,
        },
        Port {
            signal: Some(din.clone()),
            direction: port::Direction::Input as i32,
        },
        Port {
            signal: Some(dinb.clone()),
            direction: port::Direction::Output as i32,
        },
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    m.instances.push(
        Mosfet {
            name: "n".to_string(),
            width: size.nmos_width,
            length,
            drain: sig_conn(&dinb),
            source: sig_conn(&gnd),
            gate: sig_conn(&din),
            body: sig_conn(&gnd),
            mos_type: MosType::Nmos,
        }
        .into(),
    );
    m.instances.push(
        Mosfet {
            name: "p".to_string(),
            width: size.pmos_width,
            length,
            drain: sig_conn(&dinb),
            source: sig_conn(&vdd),
            gate: sig_conn(&din),
            body: sig_conn(&vdd),
            mos_type: MosType::Pmos,
        }
        .into(),
    );

    m
}
