use magic_vlsi::units::{Distance, Vec2};
use magic_vlsi::MagicInstance;
use micro_hdl::{context::Context, node::Node};

use crate::cells::gates::inv::Inv;
use crate::cells::gates::nand3::Nand3;
use crate::config::TechConfig;
use crate::error::Result;

use crate::net_name_bar;

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
                .size(crate::cells::gates::GateSize::minimum())
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

pub fn generate_predecoder2_4(m: &mut MagicInstance, _tc: &TechConfig) -> Result<()> {
    let nand2_pm_sh = m.load_layout_cell("nand2_pm_sh")?;
    let inv_pm_sh = m.load_layout_cell("inv_pm_sh_2")?;

    let cell_name = String::from("predecoder2_4");
    m.drc_off()?;
    m.load(&cell_name)?;
    m.enable_box()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    let mut height = Distance::zero();
    let mut nands = Vec::with_capacity(4);
    for i in 0..4 {
        let nand2 =
            m.place_layout_cell(nand2_pm_sh.clone(), Vec2::new(Distance::zero(), height))?;
        let inv = m.place_layout_cell(inv_pm_sh.clone(), nand2.bbox().lr())?;
        height = nand2.bbox().top_edge();
        if i == 3 {
            m.rename_cell_pin(&inv, "VPWR", "VPWR0")?;
            m.rename_cell_pin(&nand2, "VPWR", "VPWR1")?;
            m.rename_cell_pin(&inv, "VGND", "VGND0")?;
            m.rename_cell_pin(&nand2, "VGND", "VGND1")?;
        }
        nands.push(nand2);
        m.rename_cell_pin(&inv, "Y", &format!("predecode{}", i))?;
    }

    for (i, gate) in nands.iter().enumerate() {
        m.rename_cell_pin(gate, "A", &net_name_bar("addr0", 1 - (i % 2) != 0))?;
        m.rename_cell_pin(gate, "B", &net_name_bar("addr1", 1 - (i / 2) != 0))?;
    }

    m.save(&cell_name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use micro_hdl::{backend::spice::SpiceBackend, frontend::parse};

    use super::Predecoder38;

    #[test]
    fn test_netlist_predecoder38() -> Result<(), Box<dyn std::error::Error>> {
        let tree = parse(Predecoder38::top());
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
