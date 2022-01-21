use crate::{
    Context, InstancePin, Module, ModuleConfig, ModuleInstance, ModulePin, PinType, RawModule,
    Signal,
};

pub struct Resistor {
    pub value: i64,
    pub a: Signal,
    pub b: Signal,
}

impl Module for Resistor {}

impl ModuleInstance for Resistor {
    fn params(&self) -> u64 {
        self.value as u64
    }

    fn config(&self) -> ModuleConfig {
        ModuleConfig::Raw
    }

    fn generate(&self, c: &mut Context) -> Vec<InstancePin> {
        vec![]
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

    fn name(&self) -> String {
        format!("R{}", self.value)
    }
}

impl RawModule for Resistor {
    fn spice(&self) -> String {
        format!("R1 a b {}", self.value)
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
            a: Signal { id: 0 },
            b: Signal { id: 1 },
        });
    }

    #[test]
    fn resistor_implements_module() {
        let _: Box<dyn Module> = Box::new(Resistor {
            value: 1000,
            a: Signal { id: 0 },
            b: Signal { id: 1 },
        });
    }
}
