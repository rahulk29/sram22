use crate::cells::gates::{ndiff_edge_to_gate, pdiff_edge_to_gate};
use crate::error::Result;
use crate::layout::{draw_contact, draw_contacts, ContactStack};
use magic_vlsi::units::Rect;
use magic_vlsi::{units::Distance, MagicInstance};

use crate::config::TechConfig;

use super::PrechargeSize;

pub fn generate_precharge(
    m: &mut MagicInstance,
    tc: &TechConfig,
    params: PrechargeSize,
    width: Distance,
) -> Result<()> {
    let cell_name = format!("precharge_{}", params);

    m.drc_off()?;
    m.load(&cell_name)?;
    m.enable_box()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    let pmos_width = Distance::from_nm(params.rail_pmos_width_nm);
    let pmos_pass_width = Distance::from_nm(params.pass_pmos_width_nm);

    let diff_width = 2 * ndiff_edge_to_gate(tc) + tc.layer("poly").width;

    let pdiff_box1 = Rect::center_wh(
        width / 2,
        Distance::zero(),
        diff_width,
        pmos_pass_width,
        tc.grid,
    );
    m.paint_box(pdiff_box1, "pdiff")?;

    let pdiff_box2 = Rect::ll_wh(
        pdiff_box1.left_edge(),
        pdiff_box1.top_edge() + tc.layer("pdiff").space,
        diff_width,
        pmos_width,
    );
    m.paint_box(pdiff_box2, "pdiff")?;
    let pdiff_box3 = Rect::ll_wh(
        pdiff_box2.left_edge(),
        pdiff_box2.top_edge() + tc.layer("pdiff").space,
        diff_width,
        pmos_width,
    );
    m.paint_box(pdiff_box3, "pdiff")?;

    let mut nwell_box = Rect::from_dist(
        pdiff_box1.left_edge(),
        pdiff_box1.bottom_edge(),
        pdiff_box3.right_edge(),
        pdiff_box3.top_edge(),
    )
    .grow_border(tc.layer("pdiff").enclosure("nwell"));
    nwell_box.ll.x = Distance::zero();
    nwell_box.ur.x = width;
    m.paint_box(nwell_box, "nwell")?;

    let poly_height = pmos_pass_width
        + 2 * pmos_width
        + 2 * tc.layer("pdiff").space
        + 2 * tc.layer("poly").extension("pdiff");
    let poly_box = Rect::ll_wh(
        pdiff_box1.left_edge() + pdiff_edge_to_gate(tc),
        pdiff_box1.bottom_edge() - tc.layer("poly").extension("pdiff"),
        tc.layer("poly").width,
        poly_height,
    );
    m.paint_box(poly_box, "poly")?;

    let bl_right = poly_box.left_edge() - tc.space("gate", "licon");
    let bl_box = Rect::from_dist(
        bl_right - tc.layer("li").width,
        pdiff_box1.bottom_edge(),
        bl_right,
        pdiff_box3.top_edge(),
    );

    let blb_left = poly_box.right_edge() + tc.space("gate", "licon");
    let blb_box = Rect::from_dist(
        blb_left,
        pdiff_box1.bottom_edge(),
        blb_left + tc.layer("li").width,
        pdiff_box3.top_edge(),
    );

    let viali_stack = ContactStack {
        top: "m1",
        contact_drc: "ct",
        contact_layer: "viali",
        bot: "li",
    };
    let via1_stack = ContactStack {
        top: "m2",
        contact_drc: "via1",
        contact_layer: "via1",
        bot: "m1",
    };

    let bl_pass_ct = draw_contact(m, tc, viali_stack, bl_box.overlap(pdiff_box1), true)?;
    let blb_pass_ct = draw_contact(m, tc, viali_stack, blb_box.overlap(pdiff_box1), true)?;

    let bl_rail_ct = draw_contact(m, tc, viali_stack, bl_box.overlap(pdiff_box2), true)?;
    let blb_rail_ct = draw_contact(m, tc, viali_stack, blb_box.overlap(pdiff_box3), true)?;
    draw_contact(m, tc, viali_stack, blb_box.overlap(pdiff_box2), true)?;
    let bl_vdd_ct = draw_contact(m, tc, via1_stack, blb_box.overlap(pdiff_box2), true)?;

    draw_contact(m, tc, viali_stack, bl_box.overlap(pdiff_box3), true)?;
    let blb_vdd_ct = draw_contact(m, tc, via1_stack, bl_box.overlap(pdiff_box3), true)?;

    let bl_m1_r = blb_vdd_ct.bot.left_edge() - tc.layer("m1").space;
    let bl_m1_box = Rect::from_dist(
        bl_m1_r - tc.layer("m1").width,
        blb_pass_ct.top.bottom_edge(),
        bl_m1_r,
        nwell_box.top_edge(),
    );
    m.paint_box(bl_m1_box, "m1")?;

    for li_box in [bl_box, blb_box].into_iter() {
        for diff_box in [pdiff_box1, pdiff_box2, pdiff_box3].into_iter() {
            let li_ov = li_box.overlap(diff_box);
            m.paint_box(li_ov, "li")?;
            draw_contacts(m, tc, "li", "pdiffc", "licon", "pdiff", li_ov, diff_box)?;
        }
    }
    m.select_clear()?;
    m.select_top_cell()?;

    m.port_renumber()?;
    m.save(&cell_name)?;
    m.save("precharge")?;

    Ok(())
}
