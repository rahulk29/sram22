use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vlsir::circuit::connection::Stype;
use vlsir::circuit::parameter_value::Value;
use vlsir::circuit::{port, ParameterValue};
use vlsir::reference::To;
use vlsir::{circuit, QualifiedName, Reference};

pub struct Module {
    inner: vlsir::Module,
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            inner: vlsir::Module {
                name: name.into(),
                ports: vec![],
                signals: vec![],
                instances: vec![],
                parameters: vec![],
            },
        }
    }

    #[allow(dead_code)]
    fn add_ports(&mut self, ports: &[&circuit::Port]) {
        for &port in ports {
            self.add_port(port.to_owned());
        }
    }

    fn add_ports_from_signals(&mut self, signals: &[&Signal], dir: port::Direction) {
        for &signal in signals {
            self.add_port_from_signal(signal, dir);
        }
    }

    pub fn add_ports_input(&mut self, signals: &[&Signal]) {
        self.add_ports_from_signals(signals, port::Direction::Input);
    }

    pub fn add_ports_output(&mut self, signals: &[&Signal]) {
        self.add_ports_from_signals(signals, port::Direction::Output);
    }

    pub fn add_ports_inout(&mut self, signals: &[&Signal]) {
        self.add_ports_from_signals(signals, port::Direction::Inout);
    }

    #[allow(dead_code)]
    fn add_port(&mut self, port: circuit::Port) {
        self.inner.ports.push(port);
    }

    fn add_port_from_signal(&mut self, signal: &Signal, dir: port::Direction) {
        self.inner.ports.push(circuit::Port {
            signal: Some(signal.to_owned().into()),
            direction: dir as i32,
        });
    }

    pub fn add_port_input(&mut self, signal: &Signal) {
        self.add_port_from_signal(signal, port::Direction::Input);
    }

    pub fn add_port_output(&mut self, signal: &Signal) {
        self.add_port_from_signal(signal, port::Direction::Output);
    }

    pub fn add_port_inout(&mut self, signal: &Signal) {
        self.add_port_from_signal(signal, port::Direction::Inout);
    }

    pub fn add_instance(&mut self, instance: Instance) {
        self.inner.instances.push(instance.into());
    }

    pub fn add_instances(&mut self, instances: Vec<Instance>) {
        for instance in instances {
            self.add_instance(instance);
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<vlsir::Module> for Module {
    fn into(self) -> vlsir::Module {
        self.inner
    }
}

pub struct Instance {
    inner: circuit::Instance,
}

impl Instance {
    pub fn new(name: impl Into<String>, module: Reference) -> Self {
        Self {
            inner: circuit::Instance {
                name: name.into(),
                parameters: HashMap::new(),
                module: Some(module),
                connections: HashMap::new(),
            },
        }
    }

    pub fn add_conns(&mut self, connections: &[(&str, &Signal)]) {
        for (port, signal) in connections {
            self.inner
                .connections
                .insert(port.to_string(), (*signal).to_owned().into());
        }
    }

    pub fn add_conn(&mut self, port: &str, signal: &Signal) {
        self.inner
            .connections
            .insert(port.to_string(), (*signal).to_owned().into());
    }

    pub fn add_params(&mut self, params: &[(&str, &ParameterValue)]) {
        for (name, value) in params {
            self.inner
                .parameters
                .insert(name.to_string(), (*value).to_owned());
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Instance> for Instance {
    fn into(self) -> circuit::Instance {
        self.inner
    }
}

pub fn parameter_double(value: f64) -> ParameterValue {
    ParameterValue {
        value: Some(Value::Double(value)),
    }
}

pub fn local_reference(name: impl Into<String>) -> Reference {
    Reference {
        to: Some(To::Local(name.into())),
    }
}

pub fn external_reference(domain: impl Into<String>, name: impl Into<String>) -> Reference {
    Reference {
        to: Some(To::External(QualifiedName {
            domain: domain.into(),
            name: name.into(),
        })),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SignalSlice {
    name: String,
    total_width: usize,
    start: usize,
    end: usize,
}

impl SignalSlice {
    fn width(&self) -> usize {
        assert!(self.end > self.start);
        let width = self.end - self.start;
        width as usize
    }
}

impl From<SignalSlice> for Signal {
    fn from(signal: SignalSlice) -> Self {
        Self {
            parts: vec![SignalComponent::SignalSlice(signal)],
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Connection> for &SignalSlice {
    fn into(self) -> circuit::Connection {
        if self.start == 0 && self.total_width == self.end {
            let signal: Signal = self.to_owned().into();
            circuit::Connection {
                stype: Some(Stype::Sig(signal.into())),
            }
        } else {
            let top = (self.end - 1) as i64;
            let bot = self.start as i64;

            assert!(top >= 0);
            assert!(bot >= 0);

            circuit::Connection {
                stype: Some(Stype::Slice(circuit::Slice {
                    signal: self.name.clone(),
                    top,
                    bot,
                })),
            }
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Connection> for SignalSlice {
    fn into(self) -> circuit::Connection {
        if self.start == 0 && self.total_width == self.end {
            let signal: Signal = self.into();
            circuit::Connection {
                stype: Some(Stype::Sig(signal.into())),
            }
        } else {
            let top = (self.end - 1) as i64;
            let bot = self.start as i64;

            assert!(top >= 0);
            assert!(bot >= 0);

            circuit::Connection {
                stype: Some(Stype::Slice(circuit::Slice {
                    signal: self.name,
                    top,
                    bot,
                })),
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum SignalComponent {
    Signal(Signal),
    SignalSlice(SignalSlice),
}

impl SignalComponent {
    pub fn width(&self) -> usize {
        match self {
            SignalComponent::Signal(s) => s.width(),
            SignalComponent::SignalSlice(ss) => ss.width(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Connection> for &SignalComponent {
    fn into(self) -> circuit::Connection {
        match self {
            SignalComponent::Signal(s) => s.into(),
            SignalComponent::SignalSlice(ss) => ss.into(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Connection> for SignalComponent {
    fn into(self) -> circuit::Connection {
        match self {
            SignalComponent::Signal(s) => s.into(),
            SignalComponent::SignalSlice(ss) => ss.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    parts: Vec<SignalComponent>,
}

pub fn port(signal: Signal, dir: port::Direction) -> circuit::Port {
    circuit::Port {
        signal: Some(signal.into()),
        direction: dir as i32,
    }
}

pub fn port_input(signal: Signal) -> circuit::Port {
    port(signal, port::Direction::Input)
}

pub fn port_output(signal: Signal) -> circuit::Port {
    port(signal, port::Direction::Output)
}

pub fn port_inout(signal: Signal) -> circuit::Port {
    port(signal, port::Direction::Inout)
}

pub fn signal(name: impl Into<String>) -> Signal {
    Signal {
        parts: vec![SignalComponent::SignalSlice(SignalSlice {
            name: name.into(),
            total_width: 1,
            start: 0,
            end: 1,
        })],
    }
}

pub fn bus(name: impl Into<String>, width: usize) -> Signal {
    Signal {
        parts: vec![SignalComponent::SignalSlice(SignalSlice {
            name: name.into(),
            total_width: width,
            start: 0,
            end: width,
        })],
    }
}

pub fn concat(signals: Vec<Signal>) -> Signal {
    let mut concat_signal = Signal { parts: Vec::new() };
    for mut signal in signals.into_iter() {
        if signal.parts.len() == 1 {
            concat_signal.parts.push(signal.parts.pop().unwrap());
        } else {
            concat_signal.parts.push(SignalComponent::Signal(signal));
        }
    }
    concat_signal
}

impl Signal {
    pub fn width(&self) -> usize {
        let mut width = 0;
        for part in self.parts.iter() {
            width += part.width();
        }
        width
    }

    pub fn get(&self, mut idx: usize) -> Signal {
        for part in self.parts.iter() {
            let width = part.width();
            if idx < width {
                let ret = match part {
                    SignalComponent::Signal(s) => s.get(idx),
                    SignalComponent::SignalSlice(ss) => SignalSlice {
                        name: ss.name.clone(),
                        total_width: ss.total_width,
                        start: ss.start + idx,
                        end: ss.start + idx + 1,
                    }
                    .into(),
                };
                return ret;
            }
            idx -= width;
        }
        panic!("Index out of range");
    }

    pub fn get_range(&self, mut start: usize, mut end: usize) -> Signal {
        assert!(
            start < end,
            "Range start < i < end must have nonzero elements"
        );

        let mut new_parts = vec![];
        for part in self.parts.iter() {
            let width = part.width();
            if start < width && end > 0 {
                let current_start = std::cmp::max(start, 0);
                let current_end = std::cmp::min(end, width);
                match part {
                    SignalComponent::Signal(s) => new_parts.push(SignalComponent::Signal(
                        s.get_range(current_start, current_end),
                    )),
                    SignalComponent::SignalSlice(ss) => {
                        new_parts.push(SignalComponent::SignalSlice(SignalSlice {
                            name: ss.name.clone(),
                            total_width: ss.total_width,
                            start: ss.start + current_start,
                            end: ss.start + current_end,
                        }));
                    }
                }
            }
            start = if start < width { 0 } else { start - width };
            if end < width {
                break;
            }
            end -= width;
        }

        if new_parts.is_empty() {
            panic!("Index out of range");
        }

        Signal { parts: new_parts }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Signal> for Signal {
    fn into(mut self) -> circuit::Signal {
        assert_eq!(
            self.parts.len(),
            1,
            "Empty signals and concatenated signals cannot be converted to Vlsir signals"
        );

        match self.parts.pop().unwrap() {
            SignalComponent::SignalSlice(ss) => {
                assert!(
                    ss.start == 0 && ss.total_width == ss.end,
                    "Slices of signals cannot be converted to Vlsir signals"
                );

                let width = ss.width() as i64;

                assert!(width >= 0);

                circuit::Signal {
                    name: ss.name,
                    width,
                }
            }
            SignalComponent::Signal(_) => {
                panic!("Nested signals cannot be converted to Vlsir signals")
            }
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Connection> for &Signal {
    fn into(self) -> circuit::Connection {
        assert!(
            !self.parts.is_empty(),
            "Empty signals cannot be converted to Vlsir connections"
        );

        if self.parts.len() == 1 {
            self.parts[0].clone().into()
        } else {
            circuit::Connection {
                stype: Some(Stype::Concat(circuit::Concat {
                    parts: self
                        .parts
                        .iter()
                        .rev()
                        .map(|sig_slice| sig_slice.into())
                        .collect(),
                })),
            }
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<circuit::Connection> for Signal {
    fn into(mut self) -> circuit::Connection {
        assert!(
            !self.parts.is_empty(),
            "Empty signals cannot be converted to Vlsir connections"
        );

        if self.parts.len() == 1 {
            self.parts.pop().unwrap().into()
        } else {
            circuit::Connection {
                stype: Some(Stype::Concat(circuit::Concat {
                    parts: self
                        .parts
                        .into_iter()
                        .rev()
                        .map(|sig_slice| sig_slice.into())
                        .collect(),
                })),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vlsir::circuit::port::Direction;

    use crate::schematic::vlsir_api::*;

    #[test]
    fn module_ports() {
        let name = "mock_bitcell_array";

        let vss = signal("vss");
        let bl = bus("bl", 32);
        let wl = bus("wl", 16);

        let mut module = Module::new(name);
        module.add_ports_inout(&[&vss, &bl]);
        module.add_port_input(&wl);

        let module: vlsir::Module = module.into();

        assert_eq!(module.name, name, "Module name does not match expected");
        assert_eq!(
            module.ports.len(),
            3,
            "Module does not have the correct number of ports"
        );

        let vss_port = &module.ports[0];
        let bl_port = &module.ports[1];
        let wl_port = &module.ports[2];

        let vss_signal = vss_port.signal.as_ref().unwrap();
        let bl_signal = bl_port.signal.as_ref().unwrap();
        let wl_signal = wl_port.signal.as_ref().unwrap();

        assert_eq!(
            vss_signal.name, "vss",
            "VSS port name does not match signal"
        );
        assert_eq!(
            vss_signal.width,
            vss.width() as i64,
            "VSS port width does not match signal"
        );
        assert_eq!(
            vss_port.direction,
            Direction::Inout as i32,
            "VSS port has incorrect direction"
        );

        assert_eq!(bl_signal.name, "bl", "BL port name does not match signal");
        assert_eq!(
            bl_signal.width,
            bl.width() as i64,
            "BL port width does not match signal"
        );
        assert_eq!(
            bl_port.direction,
            Direction::Inout as i32,
            "BL port has incorrect direction"
        );

        assert_eq!(wl_signal.name, "wl", "WL port name does not match signal");
        assert_eq!(
            wl_signal.width,
            wl.width() as i64,
            "WL port width does not match signal"
        );
        assert_eq!(
            wl_port.direction,
            Direction::Input as i32,
            "WL port has incorrect direction"
        );
    }

    #[test]
    fn module_add_instance() {
        let module_name = "mock_bitcell_array";
        let instance_name = "mock_bitcell";

        let mut module = Module::new(module_name);
        module.add_instance(Instance::new(instance_name, local_reference(instance_name)));

        let mut module: vlsir::Module = module.into();
        let instance = module.instances.pop().unwrap();

        assert_eq!(
            instance.name, instance_name,
            "Instance name does not match desired"
        );
        assert_eq!(
            instance.module.unwrap().to.unwrap(),
            To::Local(instance_name.to_string()),
            "Instance reference name does not match desired"
        );
    }

    #[test]
    fn instance_connections() {
        let name = "mock_bitcell";

        let vss = signal("vss");
        let bl = bus("bl", 32);
        let wl = bus("wl", 16);

        let mut instance = Instance::new(name, local_reference(name));
        instance.add_conns(&[("vss", &vss), ("bl", &bl.get(0)), ("wl", &wl.get(0))]);

        let instance: circuit::Instance = instance.into();
        assert_eq!(instance.name, name, "Instance name does not match");

        let vss_conn = instance.connections.get("vss").unwrap();
        let bl_conn = instance.connections.get("bl").unwrap();
        let wl_conn = instance.connections.get("wl").unwrap();

        assert_eq!(
            vss_conn.stype.as_ref().unwrap(),
            &Stype::Sig(circuit::Signal {
                name: "vss".to_string(),
                width: 1,
            }),
            "VSS port is connected to an incorrect signal"
        );
        assert_eq!(
            bl_conn.stype.as_ref().unwrap(),
            &Stype::Slice(circuit::Slice {
                signal: "bl".to_string(),
                top: 0,
                bot: 0,
            }),
            "BL port is connected to an incorrect signal"
        );
        assert_eq!(
            wl_conn.stype.as_ref().unwrap(),
            &Stype::Slice(circuit::Slice {
                signal: "wl".to_string(),
                top: 0,
                bot: 0,
            }),
            "WL port is connected to an incorrect signal"
        );
    }

    #[test]
    fn bus_indexing() {
        let a = bus("a", 32);
        let a5 = a.get(5);
        let a5_conn: circuit::Connection = a5.into();

        assert_eq!(
            a5_conn.stype.unwrap(),
            Stype::Slice(circuit::Slice {
                signal: "a".to_string(),
                top: 5,
                bot: 5,
            }),
            "Indexing into bus yielded incorrect result"
        );

        let a16_20 = a.get_range(16, 20);
        let a16_20_conn: circuit::Connection = a16_20.into();

        assert_eq!(
            a16_20_conn.stype.unwrap(),
            Stype::Slice(circuit::Slice {
                signal: "a".to_string(),
                top: 19,
                bot: 16,
            }),
            "Slicing into bus yielded incorrect result"
        );
    }

    #[test]
    fn concat_indexing() {
        let a = bus("a", 32);
        let b = bus("b", 16);
        let c = concat(vec![a, b]);

        let c32 = c.get(32);
        let c32_conn: circuit::Connection = c32.into();

        assert_eq!(
            c32_conn.stype.unwrap(),
            Stype::Slice(circuit::Slice {
                signal: "b".to_string(),
                top: 0,
                bot: 0,
            }),
            "Indexing into concatenation yielded incorrect result"
        );

        let c30_32 = c.get_range(30, 32);
        let c30_32_conn: circuit::Connection = c30_32.into();

        assert_eq!(
            c30_32_conn.stype.unwrap(),
            Stype::Slice(circuit::Slice {
                signal: "a".to_string(),
                top: 31,
                bot: 30,
            }),
            "Slicing into concatenation yielded incorrect result"
        );

        let c32_34 = c.get_range(32, 34);
        let c32_34_conn: circuit::Connection = c32_34.into();

        assert_eq!(
            c32_34_conn.stype.unwrap(),
            Stype::Slice(circuit::Slice {
                signal: "b".to_string(),
                top: 1,
                bot: 0,
            }),
            "Slicing into concatenation yielded incorrect result"
        );

        let c30_34 = c.get_range(30, 34);
        let c30_34_conn: circuit::Connection = c30_34.into();

        assert_eq!(
            c30_34_conn.stype.unwrap(),
            Stype::Concat(circuit::Concat {
                parts: vec![
                    circuit::Connection {
                        stype: Some(Stype::Slice(circuit::Slice {
                            signal: "b".to_string(),
                            top: 1,
                            bot: 0,
                        }))
                    },
                    circuit::Connection {
                        stype: Some(Stype::Slice(circuit::Slice {
                            signal: "a".to_string(),
                            top: 31,
                            bot: 30,
                        }))
                    },
                ]
            }),
            "Indexing into concatenation yielded incorrect result"
        );
    }
}
