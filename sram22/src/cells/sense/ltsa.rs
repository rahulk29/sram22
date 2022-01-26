use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{MosParams, Nmos, Pmos},
};

#[micro_hdl::module]
pub struct SimpleLatchTypeSenseAmplifier {
    #[input]
    sense: Node,
    #[inout]
    bl: Node,
    #[inout]
    blb: Node,
    #[inout]
    out: Node,
    #[inout]
    outb: Node,
    #[inout]
    vdd: Node,
    #[inout]
    gnd: Node,
}

impl SimpleLatchTypeSenseAmplifier {
    fn generate(ctx: &mut Context) -> SimpleLatchTypeSenseAmplifierInstance {
        let sense = ctx.node();
        let bl = ctx.node();
        let blb = ctx.node();
        let out = ctx.node();
        let outb = ctx.node();
        let vdd = ctx.node();
        let gnd = ctx.node();

        let vint = ctx.node();

        let nmos_params = MosParams {
            width_nm: 1000,
            length_nm: 150,
        };
        let pmos_params = nmos_params;

        let n1 = Nmos {
            params: nmos_params,
            d: vint,
            g: sense,
            s: gnd,
            b: gnd,
        };
        ctx.add(n1);
        let n2 = Nmos {
            params: nmos_params,
            d: out,
            g: outb,
            s: vint,
            b: gnd,
        };
        ctx.add(n2);
        let n3 = Nmos {
            params: nmos_params,
            d: outb,
            g: out,
            s: vint,
            b: gnd,
        };
        ctx.add(n3);

        let p1 = Pmos {
            params: pmos_params,
            d: out,
            g: outb,
            s: vdd,
            b: vdd,
        };
        ctx.add(p1);
        let p2 = Pmos {
            params: pmos_params,
            d: outb,
            g: out,
            s: vdd,
            b: vdd,
        };
        ctx.add(p2);
        let p3 = Pmos {
            params: pmos_params,
            d: out,
            g: sense,
            s: bl,
            b: vdd,
        };
        ctx.add(p3);
        let p4 = Pmos {
            params: pmos_params,
            d: outb,
            g: sense,
            s: blb,
            b: vdd,
        };
        ctx.add(p4);

        Self::instance()
            .sense(sense)
            .bl(bl)
            .blb(blb)
            .out(out)
            .outb(outb)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }

    fn name() -> String {
        "latch_type_sense_amp".to_string()
    }
}
