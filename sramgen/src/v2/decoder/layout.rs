use std::collections::HashSet;

use grid::Grid;
use serde::{Deserialize, Serialize};
use substrate::component::{NoParams, Component};
use substrate::index::IndexOwned;
use substrate::layout::cell::Element;
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::{Span, Point, Rect, Sign};
use substrate::layout::geom::orientation::Named;
use substrate::layout::group::elements::ElementGroup;
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::tracks::UniformTracks;
use substrate::script::Script;

use crate::v2::gate::{Gate, GateParams};

use super::{Decoder, DecoderStage};

impl Decoder {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}

impl DecoderStage {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let dsn = ctx.inner().run_script::<PhysicalDesignScript>(&NoParams)?;
        let gate = ctx.instantiate::<DecoderGate>(&self.params.gate)?;
        let row = (0..self.params.num)
            .map(|_| gate.clone().into())
            .collect::<Vec<_>>();
        let mut grid = Grid::new(0, 0);
        grid.push_row(row);
        let tiler = GridTiler::new(grid);
        ctx.draw(tiler)?;
        let tracks = UniformTracks::builder().line(dsn.hline).space(dsn.hspace).start(ctx.brect().bottom()).sign(Sign::Neg).build().unwrap();
        let hspan =  ctx.brect().hspan();
        for i in 0..self.params.num {
            ctx.draw_rect(dsn.hm, Rect::from_spans(hspan, tracks.index(i)));
        }
        Ok(())
    }
}

pub struct DecoderGate {
    pub params: GateParams,
}

impl Component for DecoderGate {
    type Params = GateParams;
    fn new(params: &Self::Params, _ctx: &substrate::data::SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self { params: params.clone() })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("decoder_gate")
    }

    fn layout(&self, ctx: &mut substrate::layout::context::LayoutCtx) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let dsn = ctx.inner().run_script::<PhysicalDesignScript>(&NoParams)?;
        let hspan = Span::until(dsn.width);
        let mut gate = ctx.instantiate::<Gate>(&self.params)?;
        gate.set_orientation(Named::R90);
        gate.place_center_x(dsn.width/2);
        ctx.add_ports(gate.ports());
        ctx.draw(gate)?;

        ctx.flatten();

        let mut group = ElementGroup::new();
        for elem in ctx.elems() {
            if dsn.abut_layers.contains(&elem.layer.layer()) {
                let rect = Rect::from_spans(hspan, elem.brect().vspan());
                group.add(Element::new(elem.layer.clone(), rect));
            }
        }

        ctx.draw(group)?;

        ctx.draw_rect(outline, Rect::from_spans(hspan, ctx.brect().vspan()));
        Ok(())
    }
}

pub struct PhysicalDesignScript;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PhysicalDesign {
    /// Width of a decoder cell.
    width: i64,
    /// Width of a decoder tap cell.
    tap_width: i64,
    /// The horizontal metal layer above (`vm`)[PhysicalDesign::vm].
    hm: LayerKey,
    /// The vertical metal layer above (`li`)[PhysicalDesign::li].
    vm: LayerKey,
    /// The metal used to connect to MOS sources, drains, gates, and taps.
    li: LayerKey,
    /// Width of wires on (`hm`)[PhysicalDesign::hm].
    hline: i64,
    /// Spacing between wires on (`hm`)[PhysicalDesign::hm].
    hspace: i64,
    /// Layers that should be extended to the edge of decoder gates and tap cells.
    abut_layers: HashSet<LayerKey>,
}

impl Script for PhysicalDesignScript {
    type Params = NoParams;
    type Output = PhysicalDesign;

    fn run(
        _params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let layers = ctx.layers();
        let li = layers.get(Selector::Metal(0))?;
        let vm = layers.get(Selector::Metal(1))?;
        let hm = layers.get(Selector::Metal(2))?;
        let nwell = layers.get(Selector::Name("nwell"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        Ok(Self::Output {
            width: 5_840,
            tap_width: 1_300,
            hm,
            vm,
            li,
            hline: 320,
            hspace: 160,
            abut_layers: HashSet::from_iter([nwell, psdm, nsdm]),
        })
    }
}
