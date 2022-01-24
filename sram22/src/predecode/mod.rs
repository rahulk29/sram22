use micro_hdl::{context::Context, node::Node};

use crate::cells::gates::inv::Inv;
use crate::cells::gates::nand3::Nand3;

#[micro_hdl::module]
pub struct Predecoder38 {
    #[input]
    addr: Vec<Node>,

    #[input]
    addr_b: Vec<Node>,

    #[output]
    decoded: Vec<Node>,

    #[inout]
    vdd: Node,
    #[inout]
    gnd: Node,
}

impl Predecoder38 {
    fn generate(c: &mut Context) -> Predecoder38Instance {
        let addr = c.bus(3);
        let addr_b = c.bus(3);
        let out = c.bus(8);
        let vdd = c.node();
        let gnd = c.node();

        for i in 0..8u16 {
            let tmp = c.node();
            let x = (0..3)
                .map(|b| (b, i & (1 << b) != 0))
                .map(|(b, x)| if x { addr[b] } else { addr_b[b] })
                .collect::<Vec<_>>();
            let nand = Nand3::instance()
                .a(x[0])
                .b(x[1])
                .c(x[2])
                .y(tmp)
                .gnd(gnd)
                .vdd(vdd)
                .build();
            c.add(nand);
            let inv = Inv::instance()
                .din(tmp)
                .dout(out[i as usize])
                .vdd(vdd)
                .gnd(gnd)
                .build();
            c.add(inv);
        }

        Predecoder38::instance()
            .addr(addr)
            .addr_b(addr_b)
            .decoded(out)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }

    fn name() -> String {
        "predecoder3_8".to_string()
    }
}

#[cfg(test)]
mod tests {
    use micro_hdl::backend::spice::SpiceBackend;

    use super::Predecoder38;

    #[test]
    fn test_predecoder38() {
        let out = <Vec<u8>>::new();
        let mut b = SpiceBackend::new(out);

        let addr = b.top_level_bus(3);
        let addr_b = b.top_level_bus(3);
        let decoded = b.top_level_bus(8);
        let vdd = b.top_level_signal();
        let gnd = b.top_level_signal();

        let predec = Predecoder38::instance()
            .addr(addr)
            .addr_b(addr_b)
            .decoded(decoded)
            .vdd(vdd)
            .gnd(gnd)
            .build();

        b.netlist(predec);
        let out = b.output();

        let out = String::from_utf8(out).unwrap();
        println!("{}", out);
    }
}
