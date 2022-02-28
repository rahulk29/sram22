use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{Flavor, Intent, Mosfet, MosfetParams},
};

#[micro_hdl::module]
pub struct Nand3 {
    #[input]
    a: Node,
    #[input]
    b: Node,
    #[input]
    c: Node,
    #[output]
    y: Node,
    #[inout]
    vdd: Node,
    #[inout]
    gnd: Node,
}

impl Nand3 {
    fn generate(ctx: &mut Context) -> Nand3Instance {
        let a = ctx.node();
        let b = ctx.node();
        let c = ctx.node();
        let y = ctx.node();
        let vdd = ctx.node();
        let gnd = ctx.node();

        let int1 = ctx.node();
        let int2 = ctx.node();

        let nmos_params = MosfetParams {
            width_nm: 1000,
            length_nm: 150,
            flavor: Flavor::Nmos,
            intent: Intent::Svt,
        };
        let pmos_params = MosfetParams {
            flavor: Flavor::Pmos,
            ..nmos_params.clone()
        };

        let n1 = Mosfet::with_params(nmos_params.clone())
            .d(int2)
            .g(c)
            .s(gnd)
            .b(gnd)
            .build();
        ctx.add_mosfet(n1);
        let n2 = Mosfet::with_params(nmos_params.clone())
            .d(int1)
            .g(b)
            .s(int2)
            .b(gnd)
            .build();
        ctx.add_mosfet(n2);
        let n3 = Mosfet::with_params(nmos_params.clone())
            .d(y)
            .g(a)
            .s(int1)
            .b(gnd)
            .build();
        ctx.add_mosfet(n3);

        let p1 = Mosfet::with_params(pmos_params.clone())
            .d(y)
            .g(a)
            .s(vdd)
            .b(vdd)
            .build();
        ctx.add_mosfet(p1);
        let p2 = Mosfet::with_params(pmos_params.clone())
            .d(y)
            .g(b)
            .s(vdd)
            .b(vdd)
            .build();
        ctx.add_mosfet(p2);
        let p3 = Mosfet::with_params(pmos_params.clone())
            .d(y)
            .g(c)
            .s(vdd)
            .b(vdd)
            .build();
        ctx.add_mosfet(p3);

        Nand3::instance()
            .a(a)
            .b(b)
            .c(c)
            .y(y)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }

    fn name() -> String {
        "nand3".to_string()
    }
}
