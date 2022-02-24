use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{MosParams, Nmos, Pmos},
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

        let nmos_params = MosParams {
            width_nm: size.nwidth_nm,
            length_nm: size.nlength_nm,
        };

        let n1 = Nmos {
            params: nmos_params,
            d: int,
            g: a,
            s: gnd,
            b: gnd,
        };
        c.add(n1);

        let n2 = Nmos {
            params: nmos_params,
            d: y,
            g: b,
            s: int,
            b: gnd,
        };
        c.add(n2);

        let pmos_params = MosParams {
            width_nm: size.pwidth_nm,
            length_nm: size.plength_nm,
        };

        let p1 = Pmos {
            params: pmos_params,
            d: y,
            g: a,
            s: vdd,
            b: vdd,
        };
        c.add(p1);

        let p2 = Pmos {
            params: pmos_params,
            d: y,
            g: b,
            s: vdd,
            b: vdd,
        };
        c.add(p2);

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
