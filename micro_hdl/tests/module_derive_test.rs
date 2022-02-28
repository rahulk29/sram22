use micro_hdl::context::Context;
use micro_hdl::node::Node;
use micro_hdl::primitive::resistor::{Resistance, Resistor};

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
            let r = Resistor::instance()
                .value(Resistance::from_kiloohms(1_000))
                .a(input[i])
                .b(output[i])
                .build();
            c.add_resistor(r);
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use super::*;
    use micro_hdl::{backend::spice::SpiceBackend, frontend::parse, transform::print_tree};

    #[test]
    fn test_print_abstract_resistor_array() {
        let tree = parse(ResistorArray::top(12));

        print_tree(&tree);
    }

    #[test]
    fn test_netlist_abstract_resistor_array() -> Result<(), Box<dyn std::error::Error>> {
        let tree = parse(ResistorArray::top(12));
        let file = tempfile::tempfile()?;
        let mut backend = SpiceBackend::with_file(file)?;
        backend.netlist(&tree)?;
        let mut file = backend.output();

        let mut s = String::new();
        file.seek(SeekFrom::Start(0))?;
        file.read_to_string(&mut s)?;
        println!("{}", &s);

        Ok(())
    }
}
