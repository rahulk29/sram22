use crate::config::TechConfig;
use crate::error::Result;
use magic_vlsi::{
    units::{Distance, Rect},
    Direction, MagicInstance,
};

pub struct InvParams {
    nmos_width: Distance,
    li: String,
    m1: String,
    height: Distance,
}

pub fn generate_pm_single_height(
    m: &mut MagicInstance,
    tc: &TechConfig,
    params: &InvParams,
) -> Result<()> {
    let _li = &params.li;
    let _m1 = &params.m1;

    let cell_name = "inv_pm_single_height";

    m.drc_off()?;
    m.load(cell_name)?;
    m.enable_box()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;
    m.scalegrid(1, 2)?;

    let pmos_width = tc.scale_pmos(params.nmos_width);

    let poly_width = params.nmos_width
        + tc.layer("poly").extension("ndiff")
        + ndiff_to_pdiff(tc)
        + pmos_width
        + tc.layer("poly").extension("pdiff");
    let poly_box = Rect::center_wh(
        Distance::zero(),
        params.height / 2,
        poly_width,
        tc.layer("poly").width,
        tc.grid,
    );
    m.paint_box(poly_box, "poly")?;

    let mut ndiff_box = poly_box;
    ndiff_box
        .shrink(Direction::Left, tc.layer("poly").extension("ndiff"))
        .grow(Direction::Up, tc.layer("ndiff").extension("poly"))
        .grow(Direction::Down, tc.layer("ndiff").extension("poly"))
        .set_width(params.nmos_width);
    m.paint_box(ndiff_box, "ndiff")?;

    let mut pdiff_box = poly_box;
    pdiff_box
        .shrink(Direction::Right, tc.layer("poly").extension("pdiff"))
        .grow(Direction::Up, tc.layer("pdiff").extension("poly"))
        .grow(Direction::Down, tc.layer("pdiff").extension("poly"))
        .set_width_from_right(pmos_width);
    m.paint_box(pdiff_box, "pdiff")?;

    let nwell_box = pdiff_box
        .clone()
        .grow_border(tc.layer("pdiff").enclosure("nwell"));
    m.paint_box(nwell_box, "nwell")?;

    m.select_top_cell()?;
    let bbox = m.select_bbox()?;

    let rail_width = bbox.width().round_to(2 * tc.grid);
    let vdd_li_box = Rect::center_wh(
        Distance::zero(),
        params.height,
        rail_width,
        tc.layer("li").width,
        tc.grid,
    );
    m.paint_box(vdd_li_box, "li")?;
    let mut vdd_m1_box = vdd_li_box;
    vdd_m1_box
        .grow(Direction::Up, tc.layer("m1").enclosure("ct"))
        .grow(Direction::Down, tc.layer("m1").enclosure("ct"));
    m.paint_box(vdd_m1_box, "m1")?;

    let gnd_li_box = Rect::center_wh(
        Distance::zero(),
        Distance::zero(),
        rail_width,
        tc.layer("li").width,
        tc.grid,
    );
    m.paint_box(gnd_li_box, "li")?;
    let mut gnd_m1_box = gnd_li_box;
    gnd_m1_box
        .grow(Direction::Up, tc.layer("m1").enclosure("ct"))
        .grow(Direction::Down, tc.layer("m1").enclosure("ct"));
    m.paint_box(gnd_m1_box, "m1")?;

    let poly_pad_h = tc.layer("licon").width + 2 * tc.layer("licon").enclosure("poly");
    let poly_pad_w = tc.layer("licon").width + 2 * tc.layer("licon").one_side_enclosure("poly");
    let poly_pad_box = Rect::ur_wh(
        poly_box.left_edge(),
        poly_box.top_edge(),
        poly_pad_w,
        poly_pad_h,
    );
    m.paint_box(poly_pad_box, "poly")?;
    m.paint_box(poly_pad_box, "li")?;

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

    m.save(cell_name)?;

    Ok(())
}

fn ndiff_to_pdiff(tc: &TechConfig) -> Distance {
    tc.space("ndiff", "nwell") + tc.layer("pdiff").enclosure("nwell")
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_generate_pm_single_height() {
        let tc = sky130_config();
        let mut m = get_magic();

        generate_pm_single_height(
            &mut m,
            &tc,
            &InvParams {
                nmos_width: Distance::from_nm(1_000),
                li: "li".to_string(),
                m1: "m1".to_string(),
                height: Distance::from_nm(1_580),
            },
        )
        .expect("failed to generate cell");
    }
}
