use crate::error::Result;
use magic_vlsi::{
    units::{Distance, Rect},
    Direction, MagicInstance,
};
use micro_hdl::{
    context::Context,
    node::Node,
    primitive::mos::{Flavor, Intent, Mosfet, MosfetParams},
};

use super::GateSize;

pub mod dec;
pub mod single_height;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct InvParams {
    pub n_width: i64,
    pub n_length: i64,
    pub p_width: i64,
    pub p_length: i64,
}

#[micro_hdl::module]
pub struct Inv {
    #[params]
    size: GateSize,
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
    fn generate(size: GateSize, ctx: &mut Context) -> InvInstance {
        let din = ctx.node();
        let dout = ctx.node();
        let vdd = ctx.node();
        let gnd = ctx.node();

        let params = MosfetParams {
            width_nm: size.nwidth_nm,
            length_nm: size.nlength_nm,
            flavor: Flavor::Nmos,
            intent: Intent::Svt,
        };
        let mn = Mosfet::with_params(params)
            .d(dout)
            .g(din)
            .s(gnd)
            .b(gnd)
            .build();
        ctx.add_mosfet(mn);

        let params = MosfetParams {
            width_nm: size.pwidth_nm,
            length_nm: size.plength_nm,
            flavor: Flavor::Pmos,
            intent: Intent::Svt,
        };

        let mp = Mosfet::with_params(params)
            .d(dout)
            .g(din)
            .s(gnd)
            .b(gnd)
            .build();
        ctx.add_mosfet(mp);

        Inv::instance()
            .size(size)
            .din(din)
            .dout(dout)
            .vdd(vdd)
            .gnd(gnd)
            .build()
    }

    fn name(size: GateSize) -> String {
        format!("inv_{}", size)
    }
}

/// Generates an inverter pitch matched to an SRAM bitcell
pub fn generate_pm(m: &mut MagicInstance) -> Result<()> {
    m.drc_off()?;
    m.load("inv_pm")?;
    m.enable_box()?;
    let lch = Distance::from_nm(150);
    let ptran_height = Distance::from_nm(280);
    let ntran_height = Distance::from_nm(280);
    let poly_overhang = Distance::from_nm(130);
    let ndiff_to_nwell = Distance::from_nm(340);
    let nwell_extension = Distance::from_nm(180);
    let pwell_extension = Distance::from_nm(130);
    let poly_height_nm = poly_overhang
        + ntran_height
        + ndiff_to_nwell
        + nwell_extension
        + ptran_height
        + poly_overhang;

    // width of ndiff/pdiff between fingers
    let diff_width_nm = Distance::from_nm(270);
    let zero = Distance::zero();

    let ndiff_box = Rect::from_dist(
        -diff_width_nm,
        poly_overhang,
        lch + diff_width_nm,
        poly_overhang + ntran_height,
    );
    m.paint_box(ndiff_box, "ndiff")?;

    let poly_box = Rect::from_dist(zero, zero, lch, poly_height_nm);
    m.paint_box(poly_box, "poly")?;

    let pdiff_box = Rect::from_dist(
        ndiff_box.ll.x,
        poly_box.ur.y - poly_overhang - ptran_height,
        ndiff_box.ur.x,
        poly_box.ur.y - poly_overhang,
    );
    m.paint_box(pdiff_box, "pdiff")?;

    let pwell_box = ndiff_box.grow_border(pwell_extension);
    m.paint_box(pwell_box, "pwell")?;

    let nwell_box = pdiff_box.grow_border(nwell_extension);
    m.paint_box(nwell_box, "nwell")?;

    let poly_li_space = Distance::from_nm(20);
    let li_width = Distance::from_nm(170);
    let gnd_li_box = Rect::from_dist(
        nwell_box.ll.x,
        poly_box.ll.y - poly_li_space - li_width,
        nwell_box.ur.x,
        poly_box.ll.y - poly_li_space,
    );
    m.paint_box(gnd_li_box, "li")?;

    m.save("inv_pm")?;

    Ok(())
}

/// Generates two inverters matched to the pitch of two SRAM cells
pub fn generate_pm_eo(m: &mut MagicInstance) -> Result<()> {
    m.drc_off()?;
    m.load("inv_pm_eo")?;
    m.enable_box()?;
    // Height-determining variables
    let ptran_height = Distance::from_nm(1265);
    let ntran_height = Distance::from_nm(825);
    let poly_overhang = Distance::from_nm(130);
    let _poly_li_space = Distance::from_nm(20);
    let nwell_extension = Distance::from_nm(180);
    let pwell_extension = Distance::from_nm(130);
    let ndiff_to_poly_pad = Distance::from_nm(110);
    let poly_pad_height = Distance::from_nm(330);
    let poly_pad_to_pdiff = Distance::from_nm(160);

    let licon_space = Distance::from_nm(170);
    let licon_size = Distance::from_nm(170);

    let poly_pad_to_contact = Distance::from_nm(80);
    let poly_cont_size = Distance::from_nm(170);

    let poly_pad_to_m1 = Distance::from_nm(50);
    let m1_line = Distance::from_nm(230);
    let m1_space = m1_line;

    // Width-determining variables
    let lch = Distance::from_nm(150);
    let diffc_to_gate = Distance::from_nm(60);
    let licon_side_enclosure = Distance::from_nm(80);
    let poly_pad_width = Distance::from_nm(270);
    let poly_cont_enclosure = Distance::from_nm(50);

    // width of ndiff/pdiff between fingers
    let diff_width_nm = diffc_to_gate + licon_size + diffc_to_gate;

    let poly_height_nm = poly_overhang
        + ntran_height
        + ndiff_to_poly_pad
        + poly_pad_height
        + poly_pad_to_pdiff
        + ptran_height
        + poly_overhang;

    let zero = Distance::zero();

    let ndiff_box = Rect::from_dist(
        -diff_width_nm,
        poly_overhang,
        lch + diff_width_nm,
        poly_overhang + ntran_height,
    );
    m.paint_box(ndiff_box, "ndiff")?;

    let poly_box = Rect::from_dist(zero, zero, lch, poly_height_nm);
    m.paint_box(poly_box, "poly")?;

    let pdiff_box = Rect::from_dist(
        ndiff_box.ll.x,
        poly_box.ur.y - poly_overhang - ptran_height,
        ndiff_box.ur.x,
        poly_box.ur.y - poly_overhang,
    );
    m.paint_box(pdiff_box, "pdiff")?;

    let pwell_box = ndiff_box.grow_border(pwell_extension);
    m.paint_box(pwell_box, "pwell")?;

    let nwell_box = pdiff_box.grow_border(nwell_extension);
    m.paint_box(nwell_box, "nwell")?;

    let poly_li_space = Distance::from_nm(20);
    let li_width = Distance::from_nm(170);
    let gnd_li_box = Rect::from_dist(
        nwell_box.ll.x,
        poly_box.ll.y - poly_li_space - li_width,
        nwell_box.ur.x,
        poly_box.ll.y - poly_li_space,
    );
    m.paint_box(gnd_li_box, "li")?;
    let vdd_li_box = Rect::from_dist(
        nwell_box.ll.x,
        poly_box.ur.y + poly_li_space,
        nwell_box.ur.x,
        poly_box.ur.y + poly_li_space + li_width,
    );
    m.paint_box(vdd_li_box, "li")?;

    let poly_pad_box = Rect::from_dist(
        -poly_pad_width,
        ndiff_box.ur.y + ndiff_to_poly_pad,
        zero,
        ndiff_box.ur.y + ndiff_to_poly_pad + poly_pad_height,
    );
    m.paint_box(poly_pad_box, "poly")?;

    // poly contact to li
    let poly_cont_box = Rect::ll_wh(
        poly_pad_box.ll.x + poly_cont_enclosure,
        poly_pad_box.ll.y + poly_pad_to_contact,
        poly_cont_size,
        poly_cont_size,
    );
    m.paint_box(poly_cont_box, "polycont")?;

    let mut li_box = poly_cont_box;
    li_box.grow(Direction::Right, licon_side_enclosure);
    li_box.grow(Direction::Left, Distance::from_nm(1000));
    m.paint_box(li_box, "li")?;

    // connect nmos/pmos drains
    let li_box = Rect::from_dist(
        poly_box.ur.x + diffc_to_gate,
        gnd_li_box.ur.y + licon_space,
        poly_box.ur.x + diffc_to_gate + licon_size,
        vdd_li_box.ll.y - licon_space,
    );
    m.paint_box(li_box, "li")?;

    let mut licon_area = li_box.overlap(ndiff_box);
    licon_area.shrink(Direction::Down, licon_side_enclosure);
    licon_area.shrink(Direction::Up, licon_side_enclosure);
    m.draw_contacts_y("ndiffc", licon_area, licon_size, licon_size)?;

    let mut licon_area = li_box.overlap(pdiff_box);
    licon_area.shrink(Direction::Up, licon_side_enclosure);
    licon_area.shrink(Direction::Down, licon_side_enclosure);
    m.draw_contacts_y("pdiffc", licon_area, licon_size, licon_size)?;

    // gnd to nmos source
    let li_box = Rect::from_dist(
        poly_box.ll.x - diffc_to_gate - licon_size,
        gnd_li_box.ll.y,
        poly_box.ll.x - diffc_to_gate,
        ndiff_box.ur.y,
    );
    m.paint_box(li_box, "li")?;

    let mut licon_area = li_box.overlap(ndiff_box);
    licon_area.shrink(Direction::Up, licon_side_enclosure);
    licon_area.shrink(Direction::Down, licon_side_enclosure);
    m.draw_contacts_y("ndiffc", licon_area, licon_size, licon_size)?;

    // vdd to pmos source
    let li_box = Rect::from_dist(
        poly_box.ll.x - diffc_to_gate - licon_size,
        pdiff_box.ll.y,
        poly_box.ll.x - diffc_to_gate,
        vdd_li_box.ur.y,
    );
    m.paint_box(li_box, "li")?;

    let mut licon_area = li_box.overlap(pdiff_box);
    licon_area.shrink(Direction::Up, licon_side_enclosure);
    licon_area.shrink(Direction::Down, licon_side_enclosure);
    m.draw_contacts_y("pdiffc", licon_area, licon_size, licon_size)?;

    m.select_top_cell()?;
    let bounds = m.select_bbox()?;
    let m1_box_1 = Rect::from_dist(
        bounds.ll.x,
        poly_pad_box.ll.y + poly_pad_to_m1 - m1_line,
        bounds.ur.x,
        poly_pad_box.ll.y + poly_pad_to_m1,
    );
    m.paint_box(m1_box_1, "m1")?;

    let mut m1_box_2 = m1_box_1;
    m1_box_2.translate(Direction::Up, m1_line + m1_space);
    m.paint_box(m1_box_2, "m1")?;

    m.save("inv_pm_eo")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cells::gates::GateSize;
    use std::io::{Read, Seek, SeekFrom};

    use super::Inv;
    use micro_hdl::{backend::spice::SpiceBackend, frontend::parse};

    #[test]
    fn test_netlist_inv() -> Result<(), Box<dyn std::error::Error>> {
        let tree = parse(Inv::top(GateSize::minimum()));
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
