use substrate::layout::cell::{CellPort, Port};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::geom::bbox::BoundBox;
use substrate::layout::geom::{Rect, Span};
use substrate::layout::layers::selector::Selector;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

use super::{And2, Inv, Nand2, Nand3, Nor2};

impl And2 {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}

impl Inv {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}

impl Nand2 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let db = ctx.mos_db();
        let nmos = db
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())
            .unwrap();
        let pmos = db
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())
            .unwrap();

        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![1], vec![]],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: self.params.nwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 2,
                    id: nmos.id(),
                },
                MosParams {
                    w: self.params.pwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 2,
                    id: pmos.id(),
                },
            ],
        };
        let mos = ctx.instantiate::<LayoutMos>(&params)?;
        ctx.draw_ref(&mos)?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;

        let a = mos.port("sd_0_2")?.largest_rect(m0)?;
        let b = mos.port("sd_1_2")?.largest_rect(m0)?;
        let c = mos.port("sd_1_0")?.largest_rect(m0)?;

        let out = a.bbox().union(b.bbox()).into_rect();
        ctx.add_port(CellPort::with_shape("y", m0, out));
        ctx.draw_rect(m0, out);

        let space = Span::new(a.right(), c.left());
        let vspan = Span::new(a.top(), c.bottom());
        let hspan = Span::from_center_span_gridded(space.center(), 170, 5);
        ctx.draw_rect(m0, Rect::from_spans(hspan, vspan));
        ctx.draw_rect(
            m0,
            Rect::from_spans(Span::new(hspan.start(), c.right()), c.vspan()),
        );

        ctx.add_port(mos.port("gate_0")?.into_cell_port().named("a"));
        ctx.add_port(mos.port("gate_1")?.into_cell_port().named("b"));
        ctx.add_port(mos.port("sd_0_0")?.into_cell_port().named("vss"));
        ctx.add_port(mos.port("sd_1_1")?.into_cell_port().named("vdd"));

        ctx.flatten();

        Ok(())
    }
}

impl Nand3 {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}

impl Nor2 {
    pub(crate) fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }
}
