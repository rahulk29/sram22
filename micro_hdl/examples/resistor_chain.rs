use micro_hdl::backend::spice::SpiceBackend;
use micro_hdl::context::Context;
use micro_hdl::node::Node;
use micro_hdl::primitive::resistor::Resistor;
use micro_hdl::ModuleInstance;

pub fn main() {
    let dump = Vec::new();
    let mut backend = SpiceBackend::new(dump);
    let input = backend.top_level_bus(4);
    let output = backend.top_level_bus(4);
    let top = ResistorArrayInstance {
        width: 4,
        input,
        output,
    };
    backend.netlist(top);
    let result = backend.output();

    print!("{}", std::str::from_utf8(&result).unwrap());
}

#[derive(ModuleInstance)]
pub struct ResistorChain {
    #[params]
    stages: usize,
    #[input]
    input: Node,
    #[output]
    output: Node,
}

#[derive(ModuleInstance)]
pub struct ResistorArray {
    #[params]
    width: usize,
    #[input]
    input: Vec<Node>,
    #[output]
    output: Vec<Node>,
}

impl ResistorArray {
    fn generate(width: usize, c: &mut Context) -> ResistorArrayInstance {
        let input = c.bus(width);
        let output = c.bus(width);
        for i in 0..width {
            let r = ResistorChainInstance {
                stages: 10,
                input: input[i],
                output: output[i],
            };
            c.add(r);
        }
        ResistorArrayInstance {
            width,
            input,
            output,
        }
    }
}

impl ResistorChain {
    fn generate(stages: usize, c: &mut Context) -> ResistorChainInstance {
        let input = c.node();
        let output = c.node();
        let mut curr = input;
        for _ in 0..stages {
            let temp = c.node();
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
