use micro_hdl::context::Context;
use micro_hdl::node::Node;
use micro_hdl::primitive::resistor::Resistor;

#[micro_hdl::module]
pub struct ResistorArray {
    #[params]
    pub width: usize,

    #[input]
    pub input: Vec<Node>,

    #[output]
    pub output: Vec<Node>,
}

impl ResistorArray {
    fn generate(width: usize, c: &mut Context) -> ResistorArrayInstance {
        let input = c.bus(width);
        let output = c.bus(width);
        for i in 0..width {
            let r = ResistorModule::instance()
                .stages(4)
                .input(input[i])
                .output(output[i])
                .build();
            c.add(r);
        }

        Self::instance()
            .width(width)
            .input(input)
            .output(output)
            .build()
    }

    fn name(width: usize) -> String {
        format!("resistor_array_{}", width)
    }
}

#[micro_hdl::module]
pub struct ResistorModule {
    #[params]
    pub stages: usize,
    #[input]
    pub input: Node,
    #[output]
    pub output: Node,
}

impl ResistorModule {
    fn generate(stages: usize, c: &mut Context) -> ResistorModuleInstance {
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
        ResistorModuleInstance {
            stages,
            input,
            output,
        }
    }

    fn name(stages: usize) -> String {
        format!("resistor_module_{}", stages)
    }
}

#[test]
fn test_create_resistor_instance() {
    let _rm = ResistorModule::instance()
        .input(Node::test())
        .output(Node::test())
        .stages(12)
        .build();
}
