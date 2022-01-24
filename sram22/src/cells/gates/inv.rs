use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{MosParams, Nmos, Pmos},
};

#[micro_hdl::module]
pub struct Inv {
    #[input]
    din: Node,
    #[output]
    dout: Node,
    #[inout]
    vdd: Node,
    #[inout]
    gnd: Node,
}

impl Inv {
    fn generate(ctx: &mut Context) -> InvInstance {
        let din = ctx.node();
        let dout = ctx.node();
        let vdd = ctx.node();
        let gnd = ctx.node();

        let nmos_params = MosParams {
            width_nm: 1000,
            length_nm: 150,
        };
        let pmos_params = nmos_params;

        let n1 = Nmos {
            params: nmos_params,
            d: dout,
            g: din,
            s: gnd,
            b: gnd,
        };
        ctx.add(n1);
        let p1 = Pmos {
            params: pmos_params,
            d: dout,
            g: din,
            s: vdd,
            b: vdd,
        };
        ctx.add(p1);

        Inv::instance()
            .din(din)
            .dout(dout)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }

    fn name() -> String {
        "inv".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::Inv;
    use micro_hdl::backend::spice::SpiceBackend;

    #[test]
    fn test_netlist_inv() {
        let out = <Vec<u8>>::new();
        let mut b = SpiceBackend::new(out);

        let din = b.top_level_signal();
        let dout = b.top_level_signal();
        let vdd = b.top_level_signal();
        let gnd = b.top_level_signal();

        let inv = Inv::instance()
            .din(din)
            .dout(dout)
            .gnd(gnd)
            .vdd(vdd)
            .build();
        b.netlist(inv);
        let out = b.output();

        let out = String::from_utf8(out).unwrap();
        println!("{}", out);
    }
}
