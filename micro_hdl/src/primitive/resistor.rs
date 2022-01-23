use crate::{Context, Module, ModuleConfig, ModuleInstance, Node, PinType, Port, Signal};

pub struct Resistor {
    pub value: i64,
    pub a: Node,
    pub b: Node,
}

impl Module for Resistor {}

impl ModuleInstance for Resistor {
    fn generate(&self, _c: &mut Context) -> Vec<Signal> {
        panic!("cannot generate resistor");
    }

    fn spice(&self) -> String {
        format!("R1 a b {}", self.value)
    }

    fn name(&self) -> String {
        format!("res_{}", self.value)
    }

    fn get_ports(&self) -> Vec<Port> {
        vec![
            Port {
                name: "a".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.a),
            },
            Port {
                name: "b".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.b),
            },
        ]
    }

    fn config(&self) -> ModuleConfig {
        ModuleConfig::Raw
    }
}

#[cfg(test)]
mod tests {
    use crate::{Module, ModuleInstance, Node};

    use super::Resistor;

    #[test]
    fn resistor_implements_module_instance() {
        let _: Box<dyn ModuleInstance> = Box::new(Resistor {
            value: 1000,
            a: Node::test(),
            b: Node::test(),
        });
    }

    #[test]
    fn resistor_implements_module() {
        let _: Box<dyn Module> = Box::new(Resistor {
            value: 1000,
            a: Node::test(),
            b: Node::test(),
        });
    }
}
