use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Rect, Span};
use substrate::layout::cell::{CellPort, MustConnect, Port};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::placement::align::AlignRect;
use substrate::layout::routing::manual::jog::OffsetJog;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

use super::{
    And2, And3, FoldedInv, Inv, MultiFingerInv, MultiFingerInvMosParams, Nand2, Nand3, Nor2,
};

impl And2 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let nand = ctx.instantiate::<Nand2>(&self.params.nand)?;
        let mut inv = ctx.instantiate::<FoldedInv>(&self.params.inv)?;

        inv.align_to_the_right_of(nand.bbox(), 300);
        inv.align_centers_vertically_gridded(nand.bbox(), ctx.pdk().layout_grid());

        let m0 = nand.port("y")?.any_layer();
        for dst in inv
            .port("a")?
            .shapes(m0)
            .filter_map(|shape| shape.as_rect())
        {
            let jog = OffsetJog::builder()
                .dir(subgeom::Dir::Horiz)
                .sign(subgeom::Sign::Pos)
                .src(nand.port("y")?.largest_rect(m0)?)
                .dst(dst.bottom())
                .layer(m0)
                .space(170)
                .build()
                .unwrap();
            let rect = Rect::from_spans(Span::new(jog.r2().left(), dst.right()), dst.vspan());
            ctx.draw(jog)?;
            ctx.draw_rect(m0, rect);
        }

        ctx.add_port(
            nand.port("vdd")?
                .into_cell_port()
                .merged_with(inv.port("vdd")?)
                .with_must_connect(MustConnect::Yes),
        )
        .unwrap();
        ctx.add_port(
            nand.port("vss")?
                .into_cell_port()
                .merged_with(inv.port("vss")?)
                .with_must_connect(MustConnect::Yes),
        )
        .unwrap();
        ctx.add_port(nand.port("a")?).unwrap();
        ctx.add_port(nand.port("b")?).unwrap();
        ctx.add_port(nand.port("y")?.into_cell_port().named("y_b"))
            .unwrap();
        ctx.add_port(inv.port("y")?).unwrap();

        ctx.draw_ref(&nand)?;
        ctx.draw_ref(&inv)?;

        ctx.flatten();

        Ok(())
    }
}

impl And3 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let nand = ctx.instantiate::<Nand3>(&self.params.nand)?;
        let mut inv = ctx.instantiate::<FoldedInv>(&self.params.inv)?;

        inv.align_to_the_right_of(nand.bbox(), 300);
        inv.align_centers_vertically_gridded(nand.bbox(), ctx.pdk().layout_grid());

        let m0 = nand.port("y")?.any_layer();
        for dst in inv
            .port("a")?
            .shapes(m0)
            .filter_map(|shape| shape.as_rect())
        {
            let jog = OffsetJog::builder()
                .dir(subgeom::Dir::Horiz)
                .sign(subgeom::Sign::Pos)
                .src(nand.port("y")?.largest_rect(m0)?)
                .dst(dst.bottom())
                .layer(m0)
                .space(170)
                .build()
                .unwrap();
            let rect = Rect::from_spans(Span::new(jog.r2().left(), dst.right()), dst.vspan());
            ctx.draw(jog)?;
            ctx.draw_rect(m0, rect);
        }

        ctx.add_port(
            nand.port("vdd")?
                .into_cell_port()
                .merged_with(inv.port("vdd")?)
                .with_must_connect(MustConnect::Yes),
        )
        .unwrap();
        ctx.add_port(
            nand.port("vss")?
                .into_cell_port()
                .merged_with(inv.port("vss")?)
                .with_must_connect(MustConnect::Yes),
        )
        .unwrap();
        ctx.add_port(nand.port("a")?).unwrap();
        ctx.add_port(nand.port("b")?).unwrap();
        ctx.add_port(nand.port("c")?).unwrap();
        ctx.add_port(nand.port("y")?.into_cell_port().named("y_b"))
            .unwrap();
        ctx.add_port(inv.port("y")?).unwrap();

        ctx.draw_ref(&nand)?;
        ctx.draw_ref(&inv)?;

        ctx.flatten();

        Ok(())
    }
}

impl Inv {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let db = ctx.mos_db();
        let nmos = db.default_nmos().unwrap();
        let pmos = db.default_pmos().unwrap();

        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 2],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: self.params.nwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: nmos.id(),
                },
                MosParams {
                    w: self.params.pwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 1,
                    id: pmos.id(),
                },
            ],
        };
        let mos = ctx.instantiate::<LayoutMos>(&params)?;
        ctx.draw_ref(&mos)?;

        let m0 = mos.port("gate_0")?.any_layer();

        let short = mos
            .port("sd_0_1")?
            .largest_rect(m0)?
            .bbox()
            .union(mos.port("sd_1_1")?.largest_rect(m0)?.bbox())
            .into_rect();
        ctx.draw_rect(m0, short);

        ctx.add_port(mos.port("gate_0")?.into_cell_port().named("a"))
            .unwrap();
        ctx.add_port(mos.port("sd_0_0")?.into_cell_port().named("vss"))
            .unwrap();
        ctx.add_port(mos.port("sd_1_0")?.into_cell_port().named("vdd"))
            .unwrap();
        ctx.add_port(CellPort::with_shape("y", m0, short)).unwrap();

        ctx.flatten();
        Ok(())
    }
}

impl FoldedInv {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let db = ctx.mos_db();
        let nmos = db.default_nmos().unwrap();
        let pmos = db.default_pmos().unwrap();

        let half_params = self.params.scale(0.5);

        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 3],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: half_params.nwidth,
                    l: half_params.length,
                    m: 1,
                    nf: 2,
                    id: nmos.id(),
                },
                MosParams {
                    w: half_params.pwidth,
                    l: half_params.length,
                    m: 1,
                    nf: 2,
                    id: pmos.id(),
                },
            ],
        };
        let mos = ctx.instantiate::<LayoutMos>(&params)?;
        ctx.draw_ref(&mos)?;

        let m0 = mos.port("gate_0")?.any_layer();

        let short = mos
            .port("sd_0_1")?
            .largest_rect(m0)?
            .bbox()
            .union(mos.port("sd_1_1")?.largest_rect(m0)?.bbox())
            .into_rect();
        ctx.draw_rect(m0, short);

        let m0_rect = mos
            .port("gate_0")?
            .bbox(m0)
            .union(mos.port("gate_1")?.bbox(m0))
            .into_rect();
        ctx.draw_rect(m0, m0_rect);
        ctx.add_port(CellPort::with_shape("a", m0, m0_rect))
            .unwrap();

        ctx.add_port(mos.port("sd_0_0")?.into_cell_port().named("vss"))
            .unwrap();
        ctx.merge_port(mos.port("sd_0_2")?.into_cell_port().named("vss"));
        ctx.add_port(mos.port("sd_1_0")?.into_cell_port().named("vdd"))
            .unwrap();
        ctx.merge_port(mos.port("sd_1_2")?.into_cell_port().named("vdd"));
        ctx.add_port(CellPort::with_shape("y", m0, short)).unwrap();
        ctx.flatten();
        Ok(())
    }
}

impl MultiFingerInv {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let db = ctx.mos_db();
        let nmos = db.default_nmos().unwrap();
        let pmos = db.default_pmos().unwrap();
        let MultiFingerInvMosParams {
            nmos_nf,
            pmos_nf,
            unit_width,
            length,
        } = self.mos_params();
        let nmos_layout = LayoutMosParams {
            skip_sd_metal: vec![vec![]; nmos_nf as usize],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::Merge,
            devices: vec![MosParams {
                w: unit_width,
                l: length,
                m: 1,
                nf: nmos_nf,
                id: nmos.id(),
            }],
        };
        let pmos_layout = LayoutMosParams {
            skip_sd_metal: vec![vec![]; pmos_nf as usize],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::Merge,
            devices: vec![MosParams {
                w: unit_width,
                l: length,
                m: 1,
                nf: pmos_nf,
                id: pmos.id(),
            }],
        };
        let nmos = ctx
            .instantiate::<LayoutMos>(&nmos_layout)?
            .with_orientation(Named::R90);
        let mut pmos = ctx
            .instantiate::<LayoutMos>(&pmos_layout)?
            .with_orientation(Named::R90);
        pmos.align_to_the_right_of(nmos.bbox(), 220);
        ctx.draw_ref(&nmos)?;
        ctx.draw_ref(&pmos)?;

        let m0 = nmos.port("gate")?.any_layer();
        let m0_rect = nmos
            .port("gate")?
            .bbox(m0)
            .union(pmos.port("gate")?.bbox(m0))
            .into_rect();
        ctx.draw_rect(m0, m0_rect);
        ctx.add_port(CellPort::with_shape("a", m0, m0_rect))
            .unwrap();
        for i in 0..=nmos_nf {
            if i % 2 == 0 {
                ctx.merge_port(
                    nmos.port(format!("sd_0_{i}"))?
                        .into_cell_port()
                        .named("vss"),
                );
            } else {
                ctx.merge_port(nmos.port(format!("sd_0_{i}"))?.into_cell_port().named("y"));
            }
        }
        for i in 0..=pmos_nf {
            if i % 2 == 0 {
                ctx.merge_port(
                    pmos.port(format!("sd_0_{i}"))?
                        .into_cell_port()
                        .named("vdd"),
                );
            } else {
                ctx.merge_port(pmos.port(format!("sd_0_{i}"))?.into_cell_port().named("y"));
            }
        }
        ctx.flatten();
        Ok(())
    }
}

impl Nand2 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let db = ctx.mos_db();
        let nmos = db.default_nmos().unwrap();
        let pmos = db.default_pmos().unwrap();

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

        let m0 = mos.port("sd_0_0")?.any_layer();

        let a = mos.port("sd_0_2")?.largest_rect(m0)?;
        let b = mos.port("sd_1_2")?.largest_rect(m0)?;
        let c = mos.port("sd_1_0")?.largest_rect(m0)?;

        let out = a.bbox().union(b.bbox()).into_rect();
        ctx.add_port(CellPort::with_shape("y", m0, out)).unwrap();
        ctx.draw_rect(m0, out);

        let space = Span::new(a.right(), c.left());
        let vspan = Span::new(a.top(), c.bottom());
        let hspan =
            Span::from_center_span_gridded(space.center(), a.height(), ctx.pdk().layout_grid());
        ctx.draw_rect(m0, Rect::from_spans(hspan, vspan));
        ctx.draw_rect(
            m0,
            Rect::from_spans(Span::new(hspan.start(), c.right()), c.vspan()),
        );

        ctx.add_port(mos.port("gate_0")?.into_cell_port().named("a"))
            .unwrap();
        ctx.add_port(mos.port("gate_1")?.into_cell_port().named("b"))
            .unwrap();
        ctx.add_port(mos.port("sd_0_0")?.into_cell_port().named("vss"))
            .unwrap();
        ctx.add_port(mos.port("sd_1_1")?.into_cell_port().named("vdd"))
            .unwrap();

        ctx.flatten();

        Ok(())
    }
}

impl Nand3 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let db = ctx.mos_db();
        let nmos = db.default_nmos().unwrap();
        let pmos = db.default_pmos().unwrap();

        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![1, 2], vec![]],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: self.params.nwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 3,
                    id: nmos.id(),
                },
                MosParams {
                    w: self.params.pwidth,
                    l: self.params.length,
                    m: 1,
                    nf: 3,
                    id: pmos.id(),
                },
            ],
        };
        let mos = ctx.instantiate::<LayoutMos>(&params)?;
        ctx.draw_ref(&mos)?;

        let m0 = mos.port("sd_0_0")?.any_layer();

        let a = mos.port("sd_0_3")?.largest_rect(m0)?;
        let b = mos.port("sd_1_3")?.largest_rect(m0)?;
        let c = mos.port("sd_1_1")?.largest_rect(m0)?;

        let out = a.bbox().union(b.bbox()).into_rect();
        ctx.add_port(CellPort::with_shape("y", m0, out)).unwrap();
        ctx.draw_rect(m0, out);

        let space = Span::new(a.right(), c.left());
        let vspan = Span::new(a.top(), c.bottom());
        let hspan =
            Span::from_center_span_gridded(space.center(), a.height(), ctx.pdk().layout_grid());
        ctx.draw_rect(m0, Rect::from_spans(hspan, vspan));
        ctx.draw_rect(
            m0,
            Rect::from_spans(Span::new(hspan.start(), c.right()), c.vspan()),
        );

        ctx.add_port(mos.port("gate_0")?.into_cell_port().named("a"))
            .unwrap();
        ctx.add_port(mos.port("gate_1")?.into_cell_port().named("b"))
            .unwrap();
        ctx.add_port(mos.port("gate_2")?.into_cell_port().named("c"))
            .unwrap();
        ctx.add_port(mos.port("sd_0_0")?.into_cell_port().named("vss"))
            .unwrap();
        let mut vdd_port = mos.port("sd_1_0")?.into_cell_port().named("vdd");
        vdd_port.merge(mos.port("sd_1_2")?);
        ctx.add_port(vdd_port).unwrap();

        ctx.flatten();

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
