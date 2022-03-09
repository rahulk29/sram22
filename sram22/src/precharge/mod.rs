use std::fmt::Display;

use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{Flavor, Intent, Mosfet, MosfetParams},
};

pub mod layout;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct PrechargeSize {
    pub rail_pmos_width_nm: i64,
    pub pass_pmos_width_nm: i64,
    pub pmos_length_nm: i64,
}

impl Display for PrechargeSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rail{}x{}_pass{}x{}",
            self.rail_pmos_width_nm,
            self.pmos_length_nm,
            self.pass_pmos_width_nm,
            self.pmos_length_nm,
        )
    }
}

#[micro_hdl::module]
pub struct Precharge {
    #[params]
    size: PrechargeSize,
    #[input]
    pc_en_b: Node,
    #[inout]
    bl: Node,
    #[inout]
    bl_b: Node,
    #[inout]
    vdd: Node,
}

impl Precharge {
    fn generate(size: PrechargeSize, ctx: &mut Context) -> PrechargeInstance {
        let pc_en_b = ctx.node();
        let bl = ctx.node();
        let bl_b = ctx.node();
        let vdd = ctx.node();

        let params = MosfetParams {
            width_nm: size.rail_pmos_width_nm,
            length_nm: size.pmos_length_nm,
            flavor: Flavor::Pmos,
            intent: Intent::Svt,
        };
        let mrail1 = Mosfet::with_params(params.clone())
            .d(vdd)
            .g(pc_en_b)
            .s(bl)
            .b(vdd)
            .build();
        ctx.add_mosfet(mrail1);
        let mrail2 = Mosfet::with_params(params)
            .d(vdd)
            .g(pc_en_b)
            .s(bl_b)
            .b(vdd)
            .build();
        ctx.add_mosfet(mrail2);

        let params = MosfetParams {
            width_nm: size.pass_pmos_width_nm,
            length_nm: size.pmos_length_nm,
            flavor: Flavor::Pmos,
            intent: Intent::Svt,
        };
        let mpass = Mosfet::with_params(params)
            .d(bl)
            .g(pc_en_b)
            .s(bl_b)
            .b(vdd)
            .build();
        ctx.add_mosfet(mpass);

        Precharge::instance()
            .size(size)
            .pc_en_b(pc_en_b)
            .bl(bl)
            .bl_b(bl_b)
            .vdd(vdd)
            .build()
    }

    fn name(size: PrechargeSize) -> String {
        format!("precharge_{}", size)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use super::*;
    use micro_hdl::{backend::spice::SpiceBackend, frontend::parse};

    #[test]
    fn test_netlist_precharge() -> Result<(), Box<dyn std::error::Error>> {
        let tree = parse(Precharge::top(PrechargeSize {
            rail_pmos_width_nm: 1000,
            pass_pmos_width_nm: 420,
            pmos_length_nm: 150,
        }));
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
