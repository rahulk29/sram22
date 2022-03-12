use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{Flavor, Intent, Mosfet, MosfetParams},
};

use super::GateSize;

pub mod single_height;

#[micro_hdl::module]
pub struct Nand2Gate {
    #[params]
    pub size: GateSize,
    #[input]
    pub a: Node,
    #[input]
    pub b: Node,
    #[output]
    pub y: Node,
    #[inout]
    pub gnd: Node,
    #[inout]
    pub vdd: Node,
}

impl Nand2Gate {
    fn name(size: GateSize) -> String {
        format!("nand2_{}", size)
    }

    fn generate(size: GateSize, c: &mut Context) -> Nand2GateInstance {
        let a = c.node();
        let b = c.node();
        let y = c.node();
        let int = c.node();
        let gnd = c.node();
        let vdd = c.node();

        let nmos_params = MosfetParams {
            width_nm: size.nwidth_nm,
            length_nm: size.nlength_nm,
            flavor: Flavor::Nmos,
            intent: Intent::Svt,
        };

        let n1 = Mosfet::with_params(nmos_params.clone())
            .d(int)
            .g(a)
            .s(gnd)
            .b(gnd)
            .build();
        c.add_mosfet(n1);

        let n2 = Mosfet::with_params(nmos_params)
            .d(y)
            .g(b)
            .s(int)
            .b(gnd)
            .build();
        c.add_mosfet(n2);

        let pmos_params = MosfetParams {
            width_nm: size.pwidth_nm,
            length_nm: size.plength_nm,
            flavor: Flavor::Pmos,
            intent: Intent::Svt,
        };

        let p1 = Mosfet::with_params(pmos_params.clone())
            .d(y)
            .g(a)
            .s(vdd)
            .b(vdd)
            .build();
        c.add_mosfet(p1);

        let p2 = Mosfet::with_params(pmos_params)
            .d(y)
            .g(b)
            .s(vdd)
            .b(vdd)
            .build();
        c.add_mosfet(p2);

        Nand2Gate::instance()
            .size(size)
            .a(a)
            .b(b)
            .y(y)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use crate::cells::gates::GateSize;
    use std::io::{Read, Seek, SeekFrom};

    use super::Nand2Gate;
    use micro_hdl::{backend::spice::SpiceBackend, frontend::parse};

    #[test]
    fn test_netlist_nand2() -> Result<(), Box<dyn std::error::Error>> {
        let tree = parse(Nand2Gate::top(GateSize::minimum()));
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
