use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::ring::Ring;
use subgeom::Rect;
use substrate::component::Component;
use substrate::layout::cell::CellPort;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerKey;

pub struct GuardRing {
    params: GuardRingParams,
}

pub struct GuardRingWrapper<T>
where
    T: Component,
{
    params: WrapperParams<T::Params>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperParams<T> {
    pub inner: T,
    pub enclosure: i64,
    pub h_metal: LayerKey,
    pub v_metal: LayerKey,
    pub h_width: i64,
    pub v_width: i64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuardRingParams {
    pub enclosure: Rect,
    pub h_metal: LayerKey,
    pub v_metal: LayerKey,
    pub h_width: i64,
    pub v_width: i64,
}

pub struct SupplyRings {
    pub(crate) vdd: Ring,
    pub(crate) vss: Ring,
}

pub const WIDTH_MULTIPLIER: i64 = 8;
pub const DNW_ENCLOSURE: i64 = 440;
pub const NWELL_HOLE_ENCLOSURE: i64 = 1_080;

impl<T> Component for GuardRingWrapper<T>
where
    T: Component,
    T::Params: Clone,
{
    type Params = WrapperParams<T::Params>;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("guard_ring_wrapper")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let &WrapperParams {
            enclosure,
            h_metal,
            v_metal,
            h_width,
            v_width,
            ..
        } = &self.params;
        let inst = ctx.instantiate::<T>(&self.params.inner)?;
        let brect = inst.brect();

        ctx.add_ports(inst.ports()).unwrap();
        ctx.draw(inst)?;

        let params = GuardRingParams {
            enclosure: brect.expand(enclosure),
            h_metal,
            v_metal,
            h_width,
            v_width,
        };
        let ring = ctx.instantiate::<GuardRing>(&params)?;
        ctx.add_ports(ring.ports()).unwrap();
        ctx.draw(ring)?;

        Ok(())
    }
}

impl Component for GuardRing {
    type Params = GuardRingParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("guard_ring")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let space = 2 * std::cmp::max(self.params.h_width, self.params.v_width);
        let vss_ring = Ring::builder()
            .inner(self.params.enclosure)
            .heights(self.params.h_width)
            .widths(self.params.v_width)
            .build();
        let vdd_ring = Ring::builder()
            .inner(vss_ring.inner().expand(space))
            .heights(self.params.h_width)
            .widths(self.params.v_width)
            .build();

        let rings = SupplyRings {
            vdd: vdd_ring,
            vss: vss_ring,
        };
        ctx.set_metadata(rings);

        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let dnw = layers.get(Selector::Name("dnwell"))?;
        let li = layers.get(Selector::Metal(0))?;
        let tap = layers.get(Selector::Name("tap"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;

        let via_pairs = [
            (tap, li),
            (li, self.params.v_metal),
            (self.params.v_metal, self.params.h_metal),
        ];

        for (port_name, ring, implant) in
            [("ring_vss", vss_ring, psdm), ("ring_vdd", vdd_ring, nsdm)]
        {
            for rv in ring.vrects() {
                ctx.draw_rect(self.params.v_metal, rv);
                ctx.merge_port(CellPort::with_shape(port_name, self.params.v_metal, rv));
                ctx.draw_rect(implant, rv);
            }

            for rv in ring.inner_vrects() {
                let r = rv.shrink(340);

                for &(bot, top) in via_pairs[..via_pairs.len() - 1].iter() {
                    let viap = ViaParams::builder().layers(bot, top).geometry(r, r).build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw(via)?;
                }
            }
            for rh in ring.inner_hrects() {
                let r = rh.shrink(340);
                for (bot, top) in via_pairs {
                    let viap = ViaParams::builder().layers(bot, top).geometry(r, r).build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw(via)?;
                }
            }

            for rh in ring.hrects() {
                ctx.draw_rect(self.params.h_metal, rh);
                ctx.merge_port(CellPort::with_shape(port_name, self.params.h_metal, rh));
                ctx.draw_rect(implant, rh);
                for rv in ring.vrects() {
                    let viap = ViaParams::builder()
                        .layers(self.params.v_metal, self.params.h_metal)
                        .geometry(rv, rh)
                        .build();

                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw(via)?;
                }
            }
        }

        let dnw_boundary = vdd_ring.inner().expand(NWELL_HOLE_ENCLOSURE);
        let nwell_width = DNW_ENCLOSURE + NWELL_HOLE_ENCLOSURE;
        let nwell_boundary = vdd_ring.inner().expand(nwell_width);

        for r in nwell_boundary.cutout(vdd_ring.inner()) {
            ctx.draw_rect(nwell, r);
        }
        ctx.draw_rect(dnw, dnw_boundary);

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use subgeom::Point;
    use substrate::layout::layers::selector::Selector;

    use crate::paths::out_gds;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    fn test_guard_ring() -> substrate::error::Result<()> {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_guard_ring");
        let layers = ctx.layers();

        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let params = GuardRingParams {
            enclosure: Rect::new(Point::zero(), Point::new(32_000, 20_000)),
            h_metal: m2,
            v_metal: m1,
            h_width: 1_360,
            v_width: 1_360,
        };
        ctx.write_layout::<GuardRing>(&params, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
        Ok(())
    }
}
