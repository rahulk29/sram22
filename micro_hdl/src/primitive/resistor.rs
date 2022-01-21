use crate::{
    Context, InstancePin, Module, ModuleConfig, ModuleInstance, ModulePin, PinType, Signal,
};

pub struct Resistor {
    pub value: i64,
    pub a: Signal,
    pub b: Signal,
}

impl Module for Resistor {}

impl ModuleInstance for Resistor {
    fn generate(&self, _c: &mut Context) -> Vec<InstancePin> {
        panic!("cannot generate resistor");
    }

    fn spice(&self) -> String {
        format!("R1 a b {}", self.value)
    }

    fn name(&self) -> String {
        format!("res_{}", self.value)
    }

    fn get_module_pins(&self) -> Vec<ModulePin> {
        vec![
            ModulePin {
                name: String::from("a"),
                pin_type: PinType::InOut,
            },
            ModulePin {
                name: String::from("b"),
                pin_type: PinType::InOut,
            },
        ]
    }

    fn get_instance_pins(&self) -> Vec<InstancePin> {
        vec![
            InstancePin { signal: self.a },
            InstancePin { signal: self.b },
        ]
    }

    fn config(&self) -> ModuleConfig {
        ModuleConfig::Raw
    }
}

#[cfg(test)]
mod tests {
    use crate::{Module, ModuleInstance, Signal};

    use super::Resistor;

    #[test]
    fn resistor_implements_module_instance() {
        let _: Box<dyn ModuleInstance> = Box::new(Resistor {
            value: 1000,
            a: Signal::with_id(0),
            b: Signal::with_id(1),
        });
    }

    #[test]
    fn resistor_implements_module() {
        let _: Box<dyn Module> = Box::new(Resistor {
            value: 1000,
            a: Signal::with_id(0),
            b: Signal::with_id(1),
        });
    }
}
