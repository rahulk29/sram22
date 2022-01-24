use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{MosParams, Nmos, Pmos},
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

        let nmos_params = MosParams {
            width_nm: 1000,
            length_nm: 150,
        };
        let pmos_params = nmos_params;

        let n1 = Nmos {
            params: nmos_params,
            d: int2,
            g: c,
            s: gnd,
            b: gnd,
        };
        ctx.add(n1);
        let n2 = Nmos {
            params: nmos_params,
            d: int1,
            g: b,
            s: int2,
            b: gnd,
        };
        ctx.add(n2);
        let n3 = Nmos {
            params: nmos_params,
            d: y,
            g: a,
            s: int1,
            b: gnd,
        };
        ctx.add(n3);
        let p1 = Pmos {
            params: pmos_params,
            d: y,
            g: a,
            s: vdd,
            b: vdd,
        };
        ctx.add(p1);
        let p2 = Pmos {
            params: pmos_params,
            d: y,
            g: b,
            s: vdd,
            b: vdd,
        };
        ctx.add(p2);
        let p3 = Pmos {
            params: pmos_params,
            d: y,
            g: c,
            s: vdd,
            b: vdd,
        };
        ctx.add(p3);

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
