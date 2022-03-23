use crate::cells::gates::{ndiff_edge_to_gate, pdiff_edge_to_gate};
use crate::error::Result;
use crate::factory::{BuildContext, Component};
use crate::layout::{draw_contact, draw_contacts};
use crate::names::PRECHARGE;
use magic_vlsi::units::{Rect, Vec2};
use magic_vlsi::Direction;
use magic_vlsi::{units::Distance, MagicInstance};

use crate::config::TechConfig;

use super::PrechargeSize;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PrechargeParams {
    pub sizing: PrechargeSize,
    pub width: Distance,
}

pub struct Precharge;
pub struct PrechargeCenter;

impl Component for Precharge {
    type Params = PrechargeParams;
    fn schematic(
        _ctx: crate::factory::BuildContext,
        _params: Self::Params,
    ) -> micro_hdl::context::ContextTree {
        todo!()
    }
    fn layout(
        mut ctx: crate::factory::BuildContext,
        params: Self::Params,
    ) -> crate::error::Result<crate::factory::Layout> {
        generate_precharge(
            &mut ctx.magic,
            &ctx.tc,
            ctx.name,
            params.sizing,
            params.width,
        )?;
        ctx.layout_from_default_magic()
    }
}

impl Component for PrechargeCenter {
    type Params = Distance;
    fn schematic(
        _ctx: crate::factory::BuildContext,
        _params: Self::Params,
    ) -> micro_hdl::context::ContextTree {
        todo!()
    }
    fn layout(
        mut ctx: crate::factory::BuildContext,
        params: Self::Params,
    ) -> crate::error::Result<crate::factory::Layout> {
        generate_precharge_center(&mut ctx, params)?;
        ctx.layout_from_default_magic()
    }
}

pub fn generate_precharge(
    m: &mut MagicInstance,
    tc: &TechConfig,
    name: &str,
    params: PrechargeSize,
    width: Distance,
) -> Result<()> {
    m.drc_off()?;
    m.load(name)?;
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

    let poly_box_extension = [
        tc.layer("poly").extension("pdiff"),
        tc.layer("li").space,
        tc.space("licon", "pdiff") - tc.layer("licon").enclosure("poly"),
    ]
    .into_iter()
    .max()
    .unwrap();
    let poly_height = pmos_pass_width
        + 2 * pmos_width
        + 2 * tc.layer("pdiff").space
        + tc.layer("poly").extension("pdiff")
        + poly_box_extension;
    let poly_box = Rect::ll_wh(
        pdiff_box1.left_edge() + pdiff_edge_to_gate(tc),
        pdiff_box1.bottom_edge() - poly_box_extension,
        tc.layer("poly").width,
        poly_height,
    );
    m.paint_box(poly_box, "poly")?;

    let poly_pad_h = tc.layer("licon").width + 2 * tc.layer("licon").enclosure("poly");
    let poly_pad_w = tc.layer("licon").width + 2 * tc.layer("licon").one_side_enclosure("poly");
    let poly_pad_box = Rect::ul_wh(
        Distance::zero(),
        poly_box.bottom_edge(),
        poly_pad_w,
        poly_pad_h,
    );
    let poly_pad_box = poly_pad_box.try_align_center_x(poly_box, tc.grid);
    m.paint_box(poly_pad_box, "poly")?;

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

    let _gate_ct = draw_contact(m, tc, tc.stack("polyc"), poly_pad_box, false)?;
    let gate_ct = draw_contact(m, tc, tc.stack("via1"), poly_pad_box, false)?;

    let m2_box = Rect::from_dist(
        Distance::zero(),
        gate_ct.top.bottom_edge(),
        width,
        gate_ct.top.top_edge(),
    );
    m.paint_box(m2_box, "m2")?;
    m.label_position_layer("PC_EN_BAR", Direction::Left, "m2")?;
    m.port_make_default()?;

    let bl_pass_ct = draw_contact(m, tc, tc.stack("viali"), bl_box.overlap(pdiff_box1), true)?;
    let blb_pass_ct = draw_contact(m, tc, tc.stack("viali"), blb_box.overlap(pdiff_box1), true)?;

    let bl_rail_ct = draw_contact(m, tc, tc.stack("viali"), bl_box.overlap(pdiff_box2), true)?;
    let blb_rail_ct = draw_contact(m, tc, tc.stack("viali"), blb_box.overlap(pdiff_box3), true)?;

    let bl_m1_box = Rect::btcxw(
        bl_pass_ct.top.bottom_edge(),
        nwell_box.top_edge(),
        bl_pass_ct.bot.center_x(tc.grid),
        tc.layer("m1").width,
    )
    .try_align_center_x(bl_pass_ct.top, tc.grid);
    m.paint_box(bl_m1_box, "m1")?;
    m.label_position_layer("BL", Direction::Up, "m1")?;
    m.port_make_default()?;

    let blb_m1_box = Rect::btcxw(
        blb_pass_ct.top.bottom_edge(),
        nwell_box.top_edge(),
        blb_pass_ct.bot.center_x(tc.grid),
        tc.layer("m1").width,
    )
    .try_align_center_x(blb_pass_ct.top, tc.grid);
    m.paint_box(blb_m1_box, "m1")?;
    m.label_position_layer("BL_BAR", Direction::Up, "m1")?;
    m.port_make_default()?;

    for li_box in [bl_box, blb_box].into_iter() {
        for diff_box in [pdiff_box1, pdiff_box2, pdiff_box3].into_iter() {
            let li_ov = li_box.overlap(diff_box);
            m.paint_box(li_ov, "li")?;
            draw_contacts(m, tc, "li", "pdiffc", "licon", "pdiff", li_ov, diff_box)?;
        }
    }

    // VDD contacts
    let bl_vdd_ct = Rect::ll_wh(
        width,
        bl_rail_ct.top.center_y(tc.grid),
        Distance::zero(),
        Distance::zero(),
    );
    let ct1 = draw_contact(m, tc, tc.stack("viali"), bl_vdd_ct, true)?;
    let ct2 = draw_contact(m, tc, tc.stack("viali"), bl_vdd_ct, true)?;
    let li_box = Rect::from_dist(
        blb_m1_box.left_edge(),
        ct1.bot.bottom_edge(),
        ct1.bot.right_edge(),
        ct1.bot.top_edge(),
    );
    m.paint_box(li_box, "li")?;
    let m2_box = Rect::from_dist(
        Distance::zero(),
        ct2.top.bottom_edge(),
        width,
        ct2.top.top_edge(),
    );
    m.paint_box(m2_box, "m2")?;
    m.label_position_layer("VPWR1", Direction::Right, "m2")?;
    m.port_make_default()?;

    let blb_vdd_ct = Rect::ll_wh(
        Distance::zero(),
        blb_rail_ct.top.center_y(tc.grid),
        Distance::zero(),
        Distance::zero(),
    );
    let ct1 = draw_contact(m, tc, tc.stack("viali"), blb_vdd_ct, true)?;
    let ct2 = draw_contact(m, tc, tc.stack("via1"), blb_vdd_ct, true)?;
    let li_box = Rect::from_dist(
        Distance::zero(),
        ct1.bot.bottom_edge(),
        bl_m1_box.right_edge(),
        ct1.bot.top_edge(),
    );
    m.paint_box(li_box, "li")?;
    let m2_box = Rect::from_dist(
        Distance::zero(),
        ct2.top.bottom_edge(),
        width,
        ct2.top.top_edge(),
    );
    m.paint_box(m2_box, "m2")?;
    m.label_position_layer("VPWR2", Direction::Left, "m2")?;
    m.port_make_default()?;

    // prune overhangs
    let erase_box = Rect::ll_wh(
        width,
        nwell_box.bottom_edge(),
        Distance::from_um(10),
        nwell_box.height(),
    );
    m.erase_box(erase_box)?;
    let erase_box = Rect::lr_wh(
        Distance::zero(),
        nwell_box.bottom_edge(),
        Distance::from_um(10),
        nwell_box.height(),
    );
    m.erase_box(erase_box)?;

    m.select_clear()?;
    m.select_top_cell()?;

    m.port_renumber()?;
    m.save(name)?;

    Ok(())
}

pub fn generate_precharge_center(ctx: &mut BuildContext, width: Distance) -> Result<()> {
    let m = &mut ctx.magic;
    let tc = &ctx.tc;

    let precharge = ctx.factory.require_layout(PRECHARGE)?.cell;
    m.drc_off()?;
    m.load(ctx.name)?;
    m.enable_box()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    let nwell_box = Rect::ll_wh(
        Distance::zero(),
        Distance::zero(),
        width,
        precharge.bbox.height(),
    );
    m.paint_box(nwell_box, "nwell")?;

    let pc = m.place_layout_cell(precharge, Vec2::new(width, Distance::zero()))?;

    for port in ["VPWR1", "VPWR2", "PC_EN_BAR"] {
        let bbox = pc.port_bbox(port);
        let m2_box = Rect::from_dist(Distance::zero(), bbox.bottom_edge(), width, bbox.top_edge());
        m.paint_box(m2_box, "m2")?;
        m.label_position_layer(port, Direction::Left, "m2")?;
        m.port_make_default()?;
    }

    let ct1 = Rect::ll_wh(
        Distance::zero(),
        pc.port_bbox("VPWR1").center_y(tc.grid),
        Distance::zero(),
        Distance::zero(),
    );
    let _ct1 = draw_contact(m, tc, tc.stack("viali"), ct1, true)?;

    let ct2 = Rect::ll_wh(
        width,
        pc.port_bbox("VPWR2").center_y(tc.grid),
        Distance::zero(),
        Distance::zero(),
    );
    let _ct2 = draw_contact(m, tc, tc.stack("viali"), ct2, true)?;

    // prune overhangs
    let delete_box = Rect::ll_wh(
        width,
        Distance::from_um(-20),
        Distance::from_um(20),
        Distance::from_um(40),
    );
    m.delete_box(delete_box)?;
    let delete_box = Rect::lr_wh(
        Distance::zero(),
        Distance::from_um(-20),
        Distance::from_um(20),
        Distance::from_um(40),
    );
    m.delete_box(delete_box)?;

    m.save(ctx.name)?;
    Ok(())
}
