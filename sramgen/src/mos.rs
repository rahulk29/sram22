use std::collections::HashMap;

use pdkprims::config::Int;
use pdkprims::mos::MosType;
use serde::{Deserialize, Serialize};
use vlsir::circuit::parameter_value::Value;
use vlsir::circuit::{port, Connection, ExternalModule, Instance, Parameter, ParameterValue, Port};
use vlsir::reference::To;
use vlsir::{QualifiedName, Reference};

use crate::utils::signal;

/// A schematic-level representation of a MOSFET.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mosfet {
    pub name: String,
    pub width: Int,
    pub length: Int,
    pub drain: Connection,
    pub source: Connection,
    pub gate: Connection,
    pub body: Connection,
    pub mos_type: pdkprims::mos::MosType,
}

pub fn ext_nmos() -> ExternalModule {
    let ports = ["d", "g", "s", "b"]
        .into_iter()
        .map(|n| Port {
            signal: Some(signal(n)),
            direction: port::Direction::Inout as i32,
        })
        .collect::<Vec<_>>();

    let parameters = ["w", "l"]
        .into_iter()
        .map(|n| Parameter {
            name: n.to_string(),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    ExternalModule {
        name: Some(QualifiedName {
            domain: "sky130".to_string(),
            name: "sky130_fd_pr__nfet_01v8".to_string(),
        }),
        desc: "A SKY130 NMOS transistor".to_string(),
        ports,
        parameters,
    }
}
pub fn ext_pmos() -> ExternalModule {
    let ports = ["d", "g", "s", "b"]
        .into_iter()
        .map(|n| Port {
            signal: Some(signal(n)),
            direction: port::Direction::Inout as i32,
        })
        .collect::<Vec<_>>();

    let parameters = ["w", "l"]
        .into_iter()
        .map(|n| Parameter {
            name: n.to_string(),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    ExternalModule {
        name: Some(QualifiedName {
            domain: "sky130".to_string(),
            name: "sky130_fd_pr__pfet_01v8".to_string(),
        }),
        desc: "A SKY130 NMOS transistor".to_string(),
        ports,
        parameters,
    }
}

impl From<Mosfet> for Instance {
    fn from(m: Mosfet) -> Self {
        let mut parameters = HashMap::new();
        parameters.insert(
            "w".to_string(),
            ParameterValue {
                value: Some(Value::Double(m.width as f64 / 1000.0)),
            },
        );
        parameters.insert(
            "l".to_string(),
            ParameterValue {
                value: Some(Value::Double(m.length as f64 / 1000.0)),
            },
        );

        let mut connections = HashMap::new();

        connections.insert("d".to_string(), m.drain);
        connections.insert("g".to_string(), m.gate);
        connections.insert("s".to_string(), m.source);
        connections.insert("b".to_string(), m.body);

        Self {
            name: m.name,
            module: Some(Reference {
                to: Some(To::External(QualifiedName {
                    domain: "sky130".to_string(),
                    name: to_name(m.mos_type),
                })),
            }),

            parameters,
            connections,
        }
    }
}

fn to_name(mos_type: pdkprims::mos::MosType) -> String {
    match mos_type {
        MosType::Nmos => "sky130_fd_pr__nfet_01v8".into(),
        MosType::Pmos => "sky130_fd_pr__pfet_01v8".into(),
    }
}
