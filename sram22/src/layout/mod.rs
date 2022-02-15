use magic_vlsi::{units::Rect, MagicInstance};

use crate::config::TechConfig;
use crate::error::Result;

pub mod bus;

#[allow(clippy::too_many_arguments)]
pub fn draw_contacts(
    m: &mut MagicInstance,
    tc: &TechConfig,
    top_layer: &str,
    contact_type: &str,
    contact_drc: &str,
    bot_layer: &str,
    top: Rect,
    bot: Rect,
) -> Result<u64> {
    let inner_top = top.grow_border(-tc.layer(contact_drc).enclosure(top_layer));
    let inner_bot = bot.grow_border(-tc.layer(contact_drc).enclosure(bot_layer));
    let mut ov = inner_top.overlap(inner_bot);
    if ov.width() > ov.height() {
        ov.ur.x = std::cmp::min(
            ov.ur.x,
            top.right_edge() - tc.layer(contact_drc).one_side_enclosure(top_layer),
        );
        ov.ll.x = std::cmp::max(
            ov.ll.x,
            top.left_edge() + tc.layer(contact_drc).one_side_enclosure(top_layer),
        );
        ov.ur.x = std::cmp::min(
            ov.ur.x,
            bot.right_edge() - tc.layer(contact_drc).one_side_enclosure(bot_layer),
        );
        ov.ll.x = std::cmp::max(
            ov.ll.x,
            bot.left_edge() + tc.layer(contact_drc).one_side_enclosure(bot_layer),
        );
    } else {
        ov.ur.y = std::cmp::min(
            ov.ur.y,
            top.top_edge() - tc.layer(contact_drc).one_side_enclosure(top_layer),
        );
        ov.ll.y = std::cmp::max(
            ov.ll.y,
            top.bottom_edge() + tc.layer(contact_drc).one_side_enclosure(top_layer),
        );
        ov.ur.y = std::cmp::min(
            ov.ur.y,
            bot.top_edge() - tc.layer(contact_drc).one_side_enclosure(bot_layer),
        );
        ov.ll.y = std::cmp::max(
            ov.ll.y,
            bot.bottom_edge() + tc.layer(contact_drc).one_side_enclosure(bot_layer),
        );
    }
    let region = ov;
    let size = tc.layer(contact_drc).width;
    let space = tc.layer(contact_drc).space;
    let nr = (region.height() + space) / (size + space);
    let nc = (region.width() + space) / (size + space);
    assert!(nr > 0, "not enough space for 1 row of contacts");
    assert!(nc > 0, "not enough space for 1 column of contacts");
    let nr = nr as u64;
    let nc = nc as u64;
    let contact_rect = Rect::ll_wh(
        region.left_edge(),
        region.bottom_edge(),
        size * nc + space * (nc - 1),
        size * nr + space * (nr - 1),
    );
    let contact_rect = contact_rect.try_align_center(ov, tc.grid);

    for i in 0..nr {
        for j in 0..nc {
            let contact_box = Rect::ll_wh(
                contact_rect.ll.x + (size + space) * j,
                contact_rect.ll.y + (size + space) * i,
                size,
                size,
            );
            m.contact(contact_box, contact_type)?;
        }
    }

    Ok(nr * nc)
}
