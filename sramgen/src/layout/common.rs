use derive_builder::Builder;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{
    AbstractPort, BoundBox, BoundBoxTrait, Cell, Element, Instance, Int, LayerKey, Rect,
};
use layout21::utils::Ptr;
use pdkprims::config::Uint;
use pdkprims::contact::ContactParams;
use pdkprims::PdkLib;
use serde::{Deserialize, Serialize};

use crate::Result;

use super::bank::GateList;

pub const NWELL_COL_SIDE_EXTEND: Int = 1_000;
pub const NWELL_COL_VERT_EXTEND: Int = 360;

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

#[derive(Copy, Clone, Builder)]
pub(crate) struct MergeArgs<'a> {
    pub(crate) layer: LayerKey,
    pub(crate) insts: GateList<'a>,
    pub(crate) port_name: &'a str,
    #[builder(default = "0")]
    pub(crate) left_overhang: isize,
    #[builder(default = "0")]
    pub(crate) right_overhang: isize,
    #[builder(default = "0")]
    pub(crate) top_overhang: isize,
    #[builder(default = "0")]
    pub(crate) bot_overhang: isize,
}

impl<'a> MergeArgs<'a> {
    #[inline]
    pub fn builder() -> MergeArgsBuilder<'static> {
        MergeArgsBuilder::default()
    }

    #[inline]
    #[allow(unused)]
    pub fn rect(self) -> Rect {
        merge(&self)
    }

    #[inline]
    pub fn element(self) -> Element {
        let rect = merge(&self);
        Element {
            net: None,
            layer: self.layer,
            purpose: layout21::raw::LayerPurpose::Drawing,
            inner: layout21::raw::Shape::Rect(rect),
        }
    }

    #[inline]
    #[allow(unused)]
    pub fn port(self) -> AbstractPort {
        let rect = merge(&self);
        let mut port = AbstractPort::new(self.port_name);
        port.add_shape(self.layer, layout21::raw::Shape::Rect(rect));
        port
    }
}

fn merge(args: &MergeArgs) -> Rect {
    let mut bbox = BoundBox::empty();
    for i in 0..args.insts.width() {
        for shape in args
            .insts
            .port(args.port_name, i)
            .shapes
            .get(&args.layer)
            .unwrap_or(&Vec::new())
            .iter()
        {
            bbox = bbox.union(&shape.bbox());
        }
    }

    let mut rect = bbox.into_rect();
    rect.p0.x -= args.left_overhang;
    rect.p0.y -= args.bot_overhang;
    rect.p1.x += args.right_overhang;
    rect.p1.y += args.top_overhang;

    rect
}
