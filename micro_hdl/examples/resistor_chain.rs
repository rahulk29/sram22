use micro_hdl::primitive::resistor::Resistor;
use micro_hdl::{
    Context, InstancePin, Module, ModuleConfig, ModuleInstance, ModulePin, PinType, Signal,
    SpiceBackend,
};

pub fn main() {
    let dump = Vec::new();
    let mut backend = SpiceBackend::new(dump);
    let input = backend.top_level_signal();
    let output = backend.top_level_signal();
    let top = ResistorChainInstance {
        stages: 8,
        input,
        output,
    };
    backend.netlist(top);
    let result = backend.output();

    print!("{}", std::str::from_utf8(&result).unwrap());
}

pub struct ResistorChain {
    stages: u32,
    input: Signal,
    output: Signal,
}

impl Module for ResistorChainInstance {}

pub struct ResistorChainInstance {
    stages: u32,
    input: Signal,
    output: Signal,
}

impl ResistorChain {
    fn generate(stages: u32, c: &mut Context) -> ResistorChainInstance {
        let input = c.signal();
        let output = c.signal();
        let mut curr = input;
        for _ in 0..stages {
            let temp = c.signal();
            let r = Resistor {
                value: 100,
                a: curr,
                b: temp,
            };
            c.add(r);
            curr = temp;
        }

        c.connect(curr, output);
        ResistorChainInstance {
            stages,
            input,
            output,
        }
    }
}

impl ModuleInstance for ResistorChainInstance {
    fn params(&self) -> u64 {
        self.stages as u64
    }

    fn name(&self) -> String {
        format!("resistor_chain_{}", self.stages)
    }

    fn generate(&self, c: &mut Context) -> Vec<InstancePin> {
        let instance = ResistorChain::generate(self.stages, c);
        vec![
            InstancePin {
                signal: instance.input,
            },
            InstancePin {
                signal: instance.output,
            },
        ]
    }

    fn get_module_pins(&self) -> Vec<ModulePin> {
        vec![
            ModulePin {
                name: String::from("input"),
                pin_type: PinType::Input,
            },
            ModulePin {
                name: String::from("output"),
                pin_type: PinType::Output,
            },
        ]
    }

    fn get_instance_pins(&self) -> Vec<InstancePin> {
        vec![
            InstancePin { signal: self.input },
            InstancePin {
                signal: self.output,
            },
        ]
    }

    fn config(&self) -> ModuleConfig {
        ModuleConfig::Generate
    }
}
