use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::{Rect, Shape, Span};
use substrate::component::Component;
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignRect;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::mos::SchematicMos;

use crate::blocks::gate::{Inv, PrimitiveGateParams};

pub struct TristateInv {
    params: PrimitiveGateParams,
}

pub struct TristateBuf {
    params: TristateBufParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TristateBufParams {
    pub inv1: PrimitiveGateParams,
    pub inv2: PrimitiveGateParams,
}

impl Component for TristateInv {
    type Params = PrimitiveGateParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("tristate_inv")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let [din, en, en_b] = ctx.ports(["din", "en", "en_b"], Direction::Input);
        let din_b = ctx.port("din_b", Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [nint, pint] = ctx.signals(["nint", "pint"]);

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();
        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?
        .named("mn_en")
        .with_connections([("d", din_b), ("g", en), ("s", nint), ("b", vss)])
        .add_to(ctx);

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?
        .named("mn_pd")
        .with_connections([("d", nint), ("g", din), ("s", vss), ("b", vss)])
        .add_to(ctx);

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?
        .named("mp_en")
        .with_connections([("d", din_b), ("g", en_b), ("s", pint), ("b", vdd)])
        .add_to(ctx);

        ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: self.params.length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?
        .named("mp_pu")
        .with_connections([("d", pint), ("g", din), ("s", vdd), ("b", vdd)])
        .add_to(ctx);

        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let poly = layers.get(Selector::Name("poly"))?;
        let db = ctx.mos_db();
        let nmos = db.default_nmos().unwrap();
        let pmos = db.default_pmos().unwrap();

        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![1]],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![MosParams {
                w: self.params.nwidth,
                l: self.params.length,
                m: 1,
                nf: 2,
                id: nmos.id(),
            }],
        };
        let pd = ctx.instantiate::<LayoutMos>(&params)?;
        ctx.draw_ref(&pd)?;

        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![1]],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![MosParams {
                w: self.params.pwidth,
                l: self.params.length,
                m: 1,
                nf: 2,
                id: pmos.id(),
            }],
        };
        let mut pu = ctx
            .instantiate::<LayoutMos>(&params)?
            .with_orientation(Named::ReflectHoriz);
        pu.align_centers_gridded(&pd, ctx.pdk().layout_grid());
        pu.align_to_the_right_of(&pd, 210);
        ctx.draw_ref(&pu)?;

        let dout_short = pd
            .port("sd_0_2")?
            .largest_rect(m0)?
            .bbox()
            .union(pu.port("sd_0_2")?.largest_rect(m0)?.bbox())
            .into_rect();

        ctx.draw_rect(m0, dout_short);
        ctx.add_port(CellPort::with_shape("dout", m0, dout_short))?;

        let mut gate_poly_spans = HashMap::new();
        for shape in pu.shapes_on(poly).chain(pd.shapes_on(poly)) {
            if let Shape::Rect(rect) = shape {
                gate_poly_spans
                    .entry(rect.vspan())
                    .or_insert(Vec::new())
                    .push(rect.hspan());
            }
        }

        for (vspan, hspans) in gate_poly_spans {
            if !vspan.intersects(&pu.port("gate_0")?.largest_rect(m0)?.vspan()) {
                continue;
            }
            let new_hspans = Span::merge_adjacent(hspans, |a, b| a.min_distance(b) < 500);
            for hspan in new_hspans {
                ctx.draw_rect(poly, Rect::from_spans(hspan, vspan));
            }
        }

        ctx.add_port(pd.port("gate_0")?.into_cell_port().named("din"))?;
        ctx.add_port(pd.port("gate_1")?.into_cell_port().named("en"))?;
        ctx.add_port(pd.port("sd_0_0")?.into_cell_port().named("vss"))?;
        ctx.add_port(pu.port("sd_0_0")?.into_cell_port().named("vdd"))?;
        ctx.merge_port(pu.port("gate_0")?.into_cell_port().named("din"));
        ctx.add_port(pu.port("gate_1")?.into_cell_port().named("en_b"))?;

        Ok(())
    }
}

impl Component for TristateBuf {
    type Params = TristateBufParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("tristate_buf")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let [din, en, en_b] = ctx.ports(["din", "en", "en_b"], Direction::Input);
        let dout = ctx.port("dout", Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let x = ctx.signal("x");

        ctx.instantiate::<Inv>(&self.params.inv1)?
            .named("inv1")
            .with_connections([("a", din), ("y", x), ("vdd", vdd), ("vss", vss)])
            .add_to(ctx);

        ctx.instantiate::<TristateInv>(&self.params.inv2)?
            .named("inv2")
            .with_connections([
                ("din", x),
                ("din_b", dout),
                ("en", en),
                ("en_b", en_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .add_to(ctx);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::blocks::gate::PrimitiveGateParams;
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::TristateInv;

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 1_000,
        pwidth: 1_800,
    };

    #[test]
    fn test_tristate_inv() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tristate_inv");
        ctx.write_schematic_to_file::<TristateInv>(&INV_SIZING, out_spice(&work_dir, "schematic"))
            .expect("failed to write schematic");
        ctx.write_layout::<TristateInv>(&INV_SIZING, out_gds(&work_dir, "layout"))
            .expect("failed to write schematic");
    }
}
