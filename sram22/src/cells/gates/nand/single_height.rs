use crate::cells::gates::{finger_space, ndiff_edge_to_gate};
use crate::error::Result;
use crate::{config::TechConfig, layout::draw_contacts};
use magic_vlsi::{
    units::{Distance, Rect},
    Direction, MagicInstance,
};

#[derive(Debug)]
pub struct Nand2Params {
    pub nmos_scale: Distance,
    pub height: Distance,
}

pub fn generate_pm_single_height(
    m: &mut MagicInstance,
    tc: &TechConfig,
    params: &Nand2Params,
) -> Result<String> {
    let cell_name = String::from("nand2_pm_sh");

    m.drc_off()?;
    m.load(&cell_name)?;
    m.enable_box()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    let pmos_width = tc.scale_pmos(params.nmos_scale);

    // multiply by 2 due to stacked devices
    let nmos_width = 2 * params.nmos_scale;

    let diff_height = 2 * ndiff_edge_to_gate(tc) + 2 * tc.layer("poly").width + finger_space(tc);

    let ndiff_box = Rect::center_wh(
        Distance::zero(),
        params.height / 2,
        nmos_width,
        diff_height,
        tc.grid,
    );
    m.paint_box(ndiff_box, "ndiff")?;

    let poly_width = nmos_width
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

    let poly_box_bot = Rect::ll_wh(
        ndiff_box.left_edge() - tc.layer("poly").extension("ndiff"),
        ndiff_box.bottom_edge() + ndiff_edge_to_gate(tc),
        poly_width,
        tc.layer("poly").width,
    );
    m.paint_box(poly_box_bot, "poly")?;
    let poly_box_top = Rect::ll_wh(
        ndiff_box.left_edge() - tc.layer("poly").extension("ndiff"),
        ndiff_box.bottom_edge()
            + ndiff_edge_to_gate(tc)
            + (tc.layer("poly").width + finger_space(tc)),
        poly_width,
        tc.layer("poly").width,
    );
    m.paint_box(poly_box_top, "poly")?;

    let poly_pad_h = tc.layer("licon").width + 2 * tc.layer("licon").enclosure("poly");
    let poly_pad_w = tc.layer("licon").width + 2 * tc.layer("licon").one_side_enclosure("poly");
    let poly_pad_box_bot = Rect::ur_wh(
        poly_box_bot.left_edge(),
        poly_box_bot.top_edge(),
        poly_pad_w,
        poly_pad_h,
    );
    m.paint_box(poly_pad_box_bot, "poly")?;
    let poly_pad_box_top = Rect::lr_wh(
        poly_box_top.left_edge(),
        poly_box_top.bottom_edge(),
        poly_pad_w,
        poly_pad_h,
    );
    m.paint_box(poly_pad_box_top, "poly")?;

    let pwell_box = Rect::from_dist(
        poly_pad_box_bot.left_edge() - tc.layer("poly").space,
        Distance::zero(),
        nwell_box.left_edge(),
        params.height,
    );
    m.paint_box(pwell_box, "pwell")?;

    for (poly_pad_box, label) in [(poly_pad_box_bot, "B"), (poly_pad_box_top, "A")] {
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

        let li_box = {
            let tmp = licon_box;
            let mut tmp = tmp.grow_border(tc.layer("licon").enclosure("li"));
            tmp.grow(Direction::Down, tc.layer("licon").one_side_enclosure("li"))
                .grow(Direction::Up, tc.layer("licon").one_side_enclosure("li"));
            tmp
        };
        m.paint_box(li_box, "li")?;

        m.label_position_layer(label, Direction::Left, "li")?;
        m.port_make_default()?;
    }

    let m1_contact_width = std::cmp::max(
        tc.layer("m1").width,
        tc.layer("ct").width + 2 * tc.layer("ct").one_side_enclosure("m1"),
    )
    .round_up_to(2 * tc.grid);

    let n_ct_top = Rect::lbrh(
        ndiff_box.left_edge(),
        poly_box_top.top_edge() + tc.space("gate", "licon"),
        nwell_box.right_edge(),
        tc.layer("li").width,
    );
    m.paint_box(n_ct_top, "li")?;
    m.label_position_layer("Y", Direction::Right, "li")?;
    m.port_make_default()?;

    draw_contacts(m, tc, "li", "ndiffc", "licon", "ndiff", n_ct_top, ndiff_box)?;
    draw_contacts(m, tc, "li", "pdiffc", "licon", "pdiff", n_ct_top, pdiff_box)?;

    let p_ct_mid = Rect::ll_wh(
        pdiff_box.left_edge(),
        poly_box_bot.top_edge() + tc.space("gate", "licon"),
        pdiff_box.width(),
        tc.layer("li").width,
    );
    draw_contacts(m, tc, "li", "pdiffc", "licon", "pdiff", p_ct_mid, pdiff_box)?;
    m.paint_box(p_ct_mid, "li")?;

    let n_ct_bot = Rect::ul_wh(
        ndiff_box.left_edge(),
        poly_box_bot.bottom_edge() - tc.space("gate", "licon"),
        ndiff_box.width(),
        tc.layer("li").width,
    );
    m.paint_box(n_ct_bot, "li")?;
    draw_contacts(m, tc, "li", "ndiffc", "licon", "ndiff", n_ct_bot, ndiff_box)?;

    let p_ct_bot = Rect::ul_wh(
        pdiff_box.left_edge(),
        poly_box_bot.bottom_edge() - tc.space("gate", "licon"),
        pdiff_box.width(),
        tc.layer("li").width,
    );
    m.paint_box(p_ct_bot, "li")?;
    draw_contacts(m, tc, "li", "pdiffc", "licon", "pdiff", p_ct_bot, pdiff_box)?;

    let cx = Distance::center_grid(ndiff_box.right_edge(), pdiff_box.left_edge(), tc.grid);
    let out_vertical_li = Rect::btcxw(
        p_ct_bot.bottom_edge(),
        n_ct_top.top_edge(),
        cx,
        tc.layer("li").width,
    );
    m.paint_box(out_vertical_li, "li")?;

    let out_horiz_li = Rect::from_dist(
        out_vertical_li.left_edge(),
        out_vertical_li.bottom_edge(),
        nwell_box.right_edge(),
        p_ct_bot.top_edge(),
    );
    m.paint_box(out_horiz_li, "li")?;

    let gnd_finger = Rect::btcxw(
        Distance::zero(),
        params.height,
        n_ct_bot.center_x(tc.grid),
        m1_contact_width,
    );
    m.paint_box(gnd_finger, "m1")?;
    m.label_position_layer("VGND", Direction::Down, "m1")?;
    m.port_make_default()?;

    draw_contacts(m, tc, "m1", "viali", "ct", "licon", gnd_finger, n_ct_bot)?;

    let vdd_finger = Rect::btcxw(
        Distance::zero(),
        params.height,
        p_ct_mid.center_x(tc.grid),
        m1_contact_width,
    );
    m.paint_box(vdd_finger, "m1")?;
    m.label_position_layer("VPWR", Direction::Up, "m1")?;
    m.port_make_default()?;

    draw_contacts(m, tc, "m1", "viali", "ct", "licon", vdd_finger, p_ct_mid)?;

    m.select_clear()?;
    m.select_top_cell()?;

    m.port_renumber()?;
    m.save(&cell_name)?;
    m.save("nand2_dec_auto")?;

    Ok(cell_name)
}

fn ndiff_to_pdiff(tc: &TechConfig) -> Distance {
    tc.space("ndiff", "nwell") + tc.layer("pdiff").enclosure("nwell")
}

#[cfg(test)]
pub mod tests {

    use super::*;

    use crate::sky130_config;
    use crate::test_utils::*;

    #[test]
    fn test_generate_nand2_pm_sh() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let tc = sky130_config();
        let mut m = get_magic();

        let _cell_name = generate_pm_single_height(
            &mut m,
            &tc,
            &Nand2Params {
                nmos_scale: Distance::from_nm(1_000),
                height: Distance::from_nm(1_580),
            },
        )
        .expect("failed to generate cell");

        /*
        let mag_path = m.getcwd().clone().join(format!("{}.mag", cell_name));

        let tree = parse(Nand2Gate::top(GateSize::minimum()));
        let file = tempfile::NamedTempFile::new()?;
        let netlist_path = file.path().to_owned();
        let mut backend = SpiceBackend::new(file);
        backend.netlist(&tree)?;

        #[cfg(feature = "netgen_lvs")]
        {
            use crate::verification::lvs::Lvs;
            use crate::verification::plugins::netgen_lvs::NetgenLvs;
            let lvs_dir = TempDir::new()?;
            let lvs_path = lvs_dir.path().to_owned();
            let _lvs_res = NetgenLvs::new().lvs(LvsInput {
                netlist: netlist_path,
                layout: mag_path,
                netlist_cell: "test".to_string(),
                layout_cell: "test".to_string(),
                work_dir: lvs_path,
                opts: NetgenLvsOpts {
                    tech: "sky130A".to_string(),
                },
            })?;
            // assert!(lvs_res.ok);
            // assert_eq!(lvs_res.errors.len(), 0);
        }
        */

        generate_pm_single_height(
            &mut m,
            &tc,
            &Nand2Params {
                nmos_scale: Distance::from_nm(2_000),
                height: Distance::from_nm(1_580),
            },
        )
        .expect("failed to generate cell");

        Ok(())
    }
}
