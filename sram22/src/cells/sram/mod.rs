#[cfg(test)]
mod tests;

use micro_hdl::{context::Context, node::Node};

#[micro_hdl::module]
pub struct Sram6T {
    #[input]
    wl: Node,
    #[inout]
    bl: Node,
    #[inout]
    blb: Node,
    #[inout]
    vdd: Node,
    #[inout]
    gnd: Node,
}

impl Sram6T {
    fn generate(ctx: &mut Context) -> Sram6TInstance {
        let wl = ctx.node();
        let bl = ctx.node();
        let blb = ctx.node();
        let vdd = ctx.node();
        let gnd = ctx.node();

        // TODO fix this
        // let q = ctx.node();
        // let qb = ctx.node();

        // let nmos_params = MosParams {
        //     width_nm: 1000,
        //     length_nm: 150,
        // };
        // let pmos_params = nmos_params;

        // let n1 = Nmos {
        //     params: nmos_params,
        //     d: q,
        //     g: qb,
        //     s: gnd,
        //     b: gnd,
        // };
        // ctx.add(n1);
        // let n2 = Nmos {
        //     params: nmos_params,
        //     d: qb,
        //     g: q,
        //     s: gnd,
        //     b: gnd,
        // };
        // ctx.add(n2);
        // let p1 = Pmos {
        //     params: pmos_params,
        //     d: q,
        //     g: qb,
        //     s: vdd,
        //     b: vdd,
        // };
        // ctx.add(p1);
        // let p2 = Pmos {
        //     params: pmos_params,
        //     d: qb,
        //     g: q,
        //     s: vdd,
        //     b: vdd,
        // };
        // ctx.add(p2);
        // let npass1 = Nmos {
        //     params: nmos_params,
        //     d: bl,
        //     g: wl,
        //     s: q,
        //     b: gnd,
        // };
        // ctx.add(npass1);
        // let npass2 = Nmos {
        //     params: nmos_params,
        //     d: blb,
        //     g: wl,
        //     s: qb,
        //     b: gnd,
        // };
        // ctx.add(npass2);

        Sram6T::instance()
            .wl(wl)
            .bl(bl)
            .blb(blb)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }

    fn name() -> String {
        "sram6t_bitcell".to_string()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ArrayDimensions {
    pub rows: usize,
    pub cols: usize,
}

#[micro_hdl::module]
pub struct BitcellArray {
    #[params]
    dims: ArrayDimensions,

    #[input]
    wordlines: Vec<Node>,

    #[output]
    bitlines: Vec<Node>,
    #[output]
    bitline_bs: Vec<Node>,

    #[inout]
    vdd: Node,
    #[inout]
    gnd: Node,
}

impl BitcellArray {
    fn generate(dims: ArrayDimensions, ctx: &mut Context) -> BitcellArrayInstance {
        let (rows, cols) = (dims.rows, dims.cols);
        let wls = ctx.bus(rows);
        let bls = ctx.bus(cols);
        let blbs = ctx.bus(cols);
        let vdd = ctx.node();
        let gnd = ctx.node();

        for wl in wls.iter() {
            for j in 0..cols {
                let cell = Sram6T::instance()
                    .wl(*wl)
                    .bl(bls[j])
                    .blb(blbs[j])
                    .vdd(vdd)
                    .gnd(gnd)
                    .build();
                ctx.add(cell);
            }
        }

        Self::instance()
            .dims(dims)
            .wordlines(wls)
            .bitlines(bls)
            .bitline_bs(blbs)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }

    fn name(dims: ArrayDimensions) -> String {
        format!("sram_array_{}_{}", dims.rows, dims.cols)
    }
}
