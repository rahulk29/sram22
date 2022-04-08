use pdkprims::config::Int;

use crate::factory::Component;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvParams {
    pub nmos_width: Int,
    pub li: String,
    pub m1: String,
    pub height: Int,
    pub fingers: usize,
}

/// A 2-finger inverter pitch matched for a single height SRAM
pub struct InvPmSh;

impl Component for InvPmSh {
    type Params = InvParams;
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
        todo!()
    }
}

/*
pub fn generate_pm_single_height(
    m: &mut MagicInstance,
    tc: &TechConfig,
    name: &str,
    params: &InvParams,
) -> Result<()> {
    let fingers = params.fingers;
    assert!(fingers >= 1, "must specify at least one finger");
    assert!(
        fingers == 1 || fingers == 2,
        "only one or two fingers supported"
    );

    let _li = &params.li;
    let _m1 = &params.m1;

    m.drc_off()?;
    m.load(name)?;
    m.enable_box()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    let pmos_width = tc.scale_pmos(params.nmos_width);

    let diff_height = 2 * ndiff_edge_to_gate(tc)
        + tc.layer("poly").width * fingers
        + finger_space(tc) * (fingers - 1);

    let ndiff_box = Rect::center_wh(
        Distance::zero(),
        params.height / 2,
        params.nmos_width,
        diff_height,
        tc.grid,
    );
    m.paint_box(ndiff_box, "ndiff")?;

    let poly_width = params.nmos_width
        + tc.layer("poly").extension("ndiff")
        + ndiff_to_pdiff(tc)
        + pmos_width
        + tc.layer("poly").extension("pdiff");

    let mut pdiff_box = ndiff_box;
    pdiff_box.translate(Direction::Right, ndiff_box.width() + ndiff_to_pdiff(tc));
    pdiff_box.set_width(pmos_width);
    m.paint_box(pdiff_box, "pdiff")?;

    let mut nwell_box = pdiff_box
        .clone()
        .grow_border(tc.layer("pdiff").enclosure("nwell"));
    nwell_box.ll.y = Distance::zero();
    nwell_box.ur.y = params.height;

    m.paint_box(nwell_box, "nwell")?;
    m.label_position_layer("VPB", Direction::Right, "nwell")?;
    m.port_make_default()?;

    let m1_contact_width = std::cmp::max(
        tc.layer("m1").width,
        tc.layer("ct").width + 2 * tc.layer("ct").one_side_enclosure("m1"),
    )
    .round_up_to(2 * tc.grid);

    let vdd_finger = Rect::btcxw(
        Distance::zero(),
        params.height,
        pdiff_box.center_x(tc.grid),
        m1_contact_width,
    );
    m.paint_box(vdd_finger, "m1")?;
    m.label_position_layer("VPWR", Direction::Up, "metal1")?;
    m.port_make_default()?;

    let gnd_finger = Rect::btcxw(
        Distance::zero(),
        params.height,
        ndiff_box.center_x(tc.grid),
        m1_contact_width,
    );
    m.paint_box(gnd_finger, "m1")?;
    m.label_position_layer("VGND", Direction::Down, "metal1")?;
    m.port_make_default()?;

    let mut gate_contact_box = Rect::zero();
    let mut poly_left = Distance::zero();
    for i in 0..fingers {
        let poly_box = Rect::ll_wh(
            ndiff_box.left_edge() - tc.layer("poly").extension("ndiff"),
            ndiff_box.bottom_edge()
                + ndiff_edge_to_gate(tc)
                + (tc.layer("poly").width + finger_space(tc)) * i,
            poly_width,
            tc.layer("poly").width,
        );
        m.paint_box(poly_box, "poly")?;
        let poly_pad_h = tc.layer("licon").width + 2 * tc.layer("licon").enclosure("poly");
        let poly_pad_w = tc.layer("licon").width + 2 * tc.layer("licon").one_side_enclosure("poly");
        let poly_pad_box = if i % 2 == 0 {
            Rect::ur_wh(
                poly_box.left_edge(),
                poly_box.top_edge(),
                poly_pad_w,
                poly_pad_h,
            )
        } else {
            Rect::lr_wh(
                poly_box.left_edge(),
                poly_box.bottom_edge(),
                poly_pad_w,
                poly_pad_h,
            )
        };
        poly_left = poly_pad_box.left_edge();
        m.paint_box(poly_pad_box, "poly")?;

        let mut licon_box = poly_pad_box;
        licon_box
            .shrink(
                Direction::Right,
                tc.layer("licon").one_side_enclosure("poly"),
            )
            .shrink(
                Direction::Left,
                tc.layer("licon").one_side_enclosure("poly"),
            )
            .shrink(Direction::Up, tc.layer("licon").enclosure("poly"))
            .shrink(Direction::Down, tc.layer("licon").enclosure("poly"));

        let li_box = if fingers == 1 {
            poly_pad_box
        } else {
            let tmp = licon_box;
            let mut tmp = tmp.grow_border(tc.layer("licon").enclosure("li"));
            tmp.grow(Direction::Down, tc.layer("licon").one_side_enclosure("li"))
                .grow(Direction::Up, tc.layer("licon").one_side_enclosure("li"));
            tmp
        };
        if gate_contact_box == Rect::zero() {
            gate_contact_box = li_box;
        } else {
            gate_contact_box.ur.y = li_box.ur.y;
        }
        m.paint_box(li_box, "li")?;

        let mut licon_box = poly_pad_box;
        licon_box
            .shrink(
                Direction::Right,
                tc.layer("licon").one_side_enclosure("poly"),
            )
            .shrink(
                Direction::Left,
                tc.layer("licon").one_side_enclosure("poly"),
            )
            .shrink(Direction::Up, tc.layer("licon").enclosure("poly"))
            .shrink(Direction::Down, tc.layer("licon").enclosure("poly"));
        m.paint_box(licon_box, "polyc")?;

        let n_ct_top = Rect::ll_wh(
            ndiff_box.left_edge(),
            poly_box.top_edge() + tc.space("gate", "licon"),
            ndiff_box.width(),
            tc.layer("li").width,
        );
        m.paint_box(n_ct_top, "li")?;
        draw_contacts(m, tc, "li", "ndiffc", "licon", "ndiff", n_ct_top, ndiff_box)?;
        let p_ct_top = Rect::ll_wh(
            pdiff_box.left_edge(),
            poly_box.top_edge() + tc.space("gate", "licon"),
            pdiff_box.width(),
            tc.layer("li").width,
        );
        draw_contacts(m, tc, "li", "pdiffc", "licon", "pdiff", p_ct_top, pdiff_box)?;
        m.paint_box(p_ct_top, "li")?;
        let n_ct_bot = Rect::ul_wh(
            ndiff_box.left_edge(),
            poly_box.bottom_edge() - tc.space("gate", "licon"),
            ndiff_box.width(),
            tc.layer("li").width,
        );
        m.paint_box(n_ct_bot, "li")?;
        draw_contacts(m, tc, "li", "ndiffc", "licon", "ndiff", n_ct_bot, ndiff_box)?;

        let p_ct_bot = Rect::ul_wh(
            pdiff_box.left_edge(),
            poly_box.bottom_edge() - tc.space("gate", "licon"),
            pdiff_box.width(),
            tc.layer("li").width,
        );
        m.paint_box(p_ct_bot, "li")?;
        draw_contacts(m, tc, "li", "pdiffc", "licon", "pdiff", p_ct_bot, pdiff_box)?;

        if i % 2 == 0 {
            let li_box = Rect::ll_wh(
                ndiff_box.left_edge(),
                poly_box.top_edge() + tc.space("gate", "licon"),
                pdiff_box.right_edge() - ndiff_box.left_edge(),
                tc.layer("li").width,
            );
            m.paint_box(li_box, "li")?;
            m.label_position_layer("Y", Direction::Right, "li")?;
            m.port_make_default()?;
        }

        let contact_target = if i % 2 == 0 { n_ct_bot } else { n_ct_top };

        draw_contacts(
            m,
            tc,
            "m1",
            "viali",
            "ct",
            "licon",
            gnd_finger,
            contact_target,
        )?;

        let contact_target = if i % 2 == 0 { p_ct_bot } else { p_ct_top };

        draw_contacts(
            m,
            tc,
            "m1",
            "viali",
            "ct",
            "licon",
            vdd_finger,
            contact_target,
        )?;
    }
    m.paint_box(gate_contact_box, "li")?;
    m.label_position_layer("A", Direction::Left, "metal1")?;
    m.port_make(0)?;

    let pwell_box = Rect::from_dist(
        poly_left - tc.layer("poly").space,
        Distance::zero(),
        nwell_box.left_edge(),
        params.height,
    );
    m.paint_box(pwell_box, "pwell")?;

    m.select_clear()?;
    m.select_top_cell()?;

    m.port_renumber()?;
    m.save(name)?;

    Ok(())
}
 */

