use derive_builder::Builder;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::BoundBoxTrait;
use layout21::{
    raw::{Cell, Instance},
    utils::Ptr,
};
use pdkprims::config::Uint;
use pdkprims::contact::ContactParams;
use pdkprims::PdkLib;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Builder)]
pub struct TwoLevelContactParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into))]
    pub bot_stack: String,
    #[builder(setter(into))]
    pub top_stack: String,
    #[builder(default = "1")]
    pub bot_rows: Uint,
    #[builder(default = "1")]
    pub bot_cols: Uint,
    #[builder(default = "1")]
    pub top_rows: Uint,
    #[builder(default = "1")]
    pub top_cols: Uint,
}

impl TwoLevelContactParams {
    #[inline]
    pub fn builder() -> TwoLevelContactParamsBuilder {
        TwoLevelContactParamsBuilder::default()
    }
}

pub fn draw_two_level_contact(
    lib: &mut PdkLib,
    params: TwoLevelContactParams,
) -> Result<Ptr<Cell>> {
    let bot = lib.pdk.get_contact(
        &ContactParams::builder()
            .stack(params.bot_stack)
            .rows(params.bot_rows)
            .cols(params.bot_cols)
            .dir(Dir::Vert)
            .build()
            .unwrap(),
    );
    let top = lib.pdk.get_contact(
        &ContactParams::builder()
            .stack(params.top_stack)
            .rows(params.top_rows)
            .cols(params.top_cols)
            .dir(Dir::Vert)
            .build()
            .unwrap(),
    );

    let bot = Instance::new("bot", bot.cell.clone());
    let mut top = Instance::new("top", top.cell.clone());
    top.align_centers_gridded(bot.bbox(), lib.pdk.grid());

    let mut p0 = bot.port("x");
    let p1 = top.port("x");

    p0.merge(p1);

    let mut cell = Cell::empty(params.name);
    cell.abs_mut().add_port(p0);
    cell.layout_mut().add_inst(bot);
    cell.layout_mut().add_inst(top);

    Ok(Ptr::new(cell))
}
