use std::collections::HashMap;

use pdkprims::config::Int;
use pdkprims::mos::MosType;
use serde::{Deserialize, Serialize};
use vlsir::circuit::parameter_value::Value;
use vlsir::circuit::{ExternalModule, Parameter, ParameterValue};
use vlsir::QualifiedName;

use crate::schematic::vlsir_api::{port_inout, signal, Instance, Signal};
use crate::schematic::NetlistFormat;

use super::vlsir_api::{external_reference, parameter_double};

/// A schematic-level representation of a MOSFET.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mosfet {
    pub name: String,
    pub width: Int,
    pub length: Int,
    pub drain: Signal,
    pub source: Signal,
    pub gate: Signal,
    pub body: Signal,
    pub mos_type: pdkprims::mos::MosType,
}

pub fn ext_nmos(format: NetlistFormat) -> ExternalModule {
    let ports = ["d", "g", "s", "b"]
        .into_iter()
        .map(|n| port_inout(signal(n)))
        .collect::<Vec<_>>();

    let parameters = ["w", "l"]
        .into_iter()
        .map(|n| Parameter {
            name: n.to_string(),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    let name = match format {
        NetlistFormat::NgSpice => "sky130_fd_pr__nfet_01v8",
        NetlistFormat::Spectre => "sky130_fd_pr__nfet_01v8",
    };

    ExternalModule {
        name: Some(QualifiedName {
            domain: "sky130".to_string(),
            name: name.to_string(),
        }),
        desc: "A SKY130 NMOS transistor".to_string(),
        ports,
        parameters,
    }
}

pub fn ext_pmos(format: NetlistFormat) -> ExternalModule {
    let ports = ["d", "g", "s", "b"]
        .into_iter()
        .map(|n| port_inout(signal(n)))
        .collect::<Vec<_>>();

    let parameters = ["w", "l"]
        .into_iter()
        .map(|n| Parameter {
            name: n.to_string(),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    let name = match format {
        NetlistFormat::NgSpice => "sky130_fd_pr__pfet_01v8",
        NetlistFormat::Spectre => "sky130_fd_pr__pfet_01v8",
    };

    ExternalModule {
        name: Some(QualifiedName {
            domain: "sky130".to_string(),
            name: name.to_string(),
        }),
        desc: "A SKY130 NMOS transistor".to_string(),
        ports,
        parameters,
    }
}

impl From<Mosfet> for Instance {
    fn from(m: Mosfet) -> Self {
        let mut inst = Self::new(m.name, external_reference("sky130", to_name(m.mos_type)));
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
        inst.add_params(&[
            ("w", &parameter_double(m.width as f64 / 1000.0)),
            ("l", &parameter_double(m.length as f64 / 1000.0)),
        ]);
        inst.add_conns(&[
            ("d", &m.drain),
            ("g", &m.gate),
            ("s", &m.source),
            ("b", &m.body),
        ]);

        inst
    }
}

fn to_name(mos_type: pdkprims::mos::MosType) -> String {
    match mos_type {
        MosType::Nmos => "sky130_fd_pr__nfet_01v8".into(),
        MosType::Pmos => "sky130_fd_pr__pfet_01v8".into(),
    }
}
