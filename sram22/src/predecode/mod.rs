use magic_vlsi::units::{Distance, Vec2};
use magic_vlsi::{Direction, MagicInstance};
use micro_hdl::{context::Context, node::Node};

use crate::cells::gates::inv::Inv;
use crate::cells::gates::nand3::Nand3;
use crate::config::TechConfig;
use crate::error::Result;
use crate::layout::bus::BusBuilder;

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

pub fn generate_predecoder2_4(m: &mut MagicInstance, tc: &TechConfig) -> Result<()> {
    let nand2_pm_sh = m.load_layout_cell("nand2_pm_sh")?;
    let inv_pm_sh = m.load_layout_cell("inv_pm_sh_2")?;

    let cell_name = String::from("predecoder2_4");
    m.drc_off()?;
    m.load(&cell_name)?;
    m.enable_box()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    let mut height = Distance::zero();
    let mut nands = Vec::with_capacity(4);
    for _ in 0..4 {
        let nand2 =
            m.place_layout_cell(nand2_pm_sh.clone(), Vec2::new(Distance::zero(), height))?;
        m.place_layout_cell(inv_pm_sh.clone(), nand2.bbox().lr())?;
        height = nand2.bbox().top_edge();
        nands.push(nand2);
    }

    let bus = BusBuilder::new()
        .width(4)
        .dir(Direction::Up)
        .tech_layer(tc, "m1")
        .allow_contact(tc, "ct", "li")
        .allow_contact(tc, "via1", "m2")
        .align_right(Distance::zero())
        .start(Distance::zero())
        .end(height)
        .draw(m)?;

    for (i, gate) in nands.iter().enumerate() {
        let target = gate.port_bbox("A");
        bus.draw_contact(m, tc, 1 - (i % 2), "ct", "viali", "li", target)?;
        let target = gate.port_bbox("B");
        bus.draw_contact(m, tc, 3 - (i / 2), "ct", "viali", "li", target)?;
    }

    m.save(&cell_name)?;

    Ok(())
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
