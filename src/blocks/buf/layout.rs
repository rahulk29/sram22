use derive_builder::Builder;

use subgeom::bbox::BoundBox;
use subgeom::transform::Transform;
use subgeom::{Corner, Dims, Dir, ExpandMode, Point, Rect, Span};
use substrate::component::Component;
use substrate::layout::cell::{CellPort, Instance, Port};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::SJog;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

use crate::blocks::gate::{FoldedInv, PrimitiveGateParams};

use super::DiffBuf;

pub const POWER_HEIGHT: i64 = 800;
pub const GRID: i64 = 5;
pub const WELL_PAD: i64 = 1_000;

impl DiffBuf {
    pub(crate) fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        todo!()
    }
}

struct Metadata {
    inv: Instance,
    vdd: Span,
    vss: Span,
}

pub struct DiffBufCent {
    params: PrimitiveGateParams,
}

impl Component for DiffBufCent {
    type Params = PrimitiveGateParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("diff_buf_cent")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let outline = layers.get(Selector::Name("outline"))?;
        let tap = layers.get(Selector::Name("tap"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let inst = ctx.instantiate::<DiffBuf>(&self.params)?;
        let meta = inst.cell().get_metadata::<Metadata>();

        let vspan = Span::new(0, 1_300);

        let mut nspan = None;
        let mut pspan = None;
        let tf = meta.inv.transformation();
        for elem in meta.inv.cell().elems() {
            let elem = elem.transform(tf);
            let layer = elem.layer.layer();
            let hspan = elem.brect().hspan();
            let rect = Rect::from_spans(hspan, vspan);
            if layer == nwell {
                ctx.draw_rect(nwell, rect);
            } else if layer == nsdm {
                pspan = Some(hspan);
                ctx.draw_rect(psdm, rect);
            } else if layer == psdm {
                nspan = Some(hspan);
                ctx.draw_rect(nsdm, rect);
            }
        }

        let nspan = nspan.unwrap();
        let pspan = pspan.unwrap();

        for (span, vdd) in [(nspan, true), (pspan, false)] {
            let r = Rect::from_spans(span, vspan).shrink(200);
            let viap = ViaParams::builder().layers(tap, m0).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            let pspan = if vdd { meta.vdd } else { meta.vss };
            let power_stripe = Rect::from_spans(pspan, vspan);

            let viap = ViaParams::builder().layers(m0, m1).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;

            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(via.layer_bbox(m1), power_stripe)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            ctx.draw_rect(m2, power_stripe);

            let name = if vdd {
                arcstr::literal!("vdd")
            } else {
                arcstr::literal!("vss")
            };
            ctx.merge_port(CellPort::with_shape(name, m2, power_stripe));
        }

        ctx.draw_rect(outline, Rect::from_spans(inst.brect().hspan(), vspan));
        Ok(())
    }
}
