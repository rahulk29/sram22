use serde::{Deserialize, Serialize};
use substrate::component::Component;
use substrate::layout::cell::{CellPort, Port};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::geom::bbox::{BoundBox, LayerBoundBox};
use substrate::layout::geom::orientation::Named;
use substrate::layout::geom::{Corner, Dir, Point, Rect, Span};
use substrate::layout::group::Group;
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::manual::jog::SJog;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffBufParams {
    pub width: i64,
    pub nw: i64,
    pub pw: i64,
    pub lch: i64,
}

pub const POWER_HEIGHT: i64 = 800;
pub const GRID: i64 = 5;

pub struct DiffBuf {
    params: DiffBufParams,
}

impl Component for DiffBuf {
    type Params = DiffBufParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("buf")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let db = ctx.mos_db();
        let nmos = db
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())
            .unwrap();
        let pmos = db
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())
            .unwrap();

        let params = LayoutMosParams {
            skip_sd_metal: vec![vec![]; 2],
            deep_nwell: true,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![
                MosParams {
                    w: self.params.nw,
                    l: self.params.lch,
                    m: 1,
                    nf: 1,
                    id: nmos.id(),
                },
                MosParams {
                    w: self.params.pw,
                    l: self.params.lch,
                    m: 1,
                    nf: 1,
                    id: pmos.id(),
                },
            ],
        };

        let mut outs = [None, None];
        let stripe_width = 340;
        let stripe_space = 160;
        let stripe_span = Span::new(-self.params.width, 2 * self.params.width);

        for j in 0..2 {
            for i in 0..2 {
                let mut inv = ctx.instantiate::<LayoutMos>(&params)?;
                inv.place_center_x(j * (inv.brect().width() + 2 * 170));
                if i == 0 {
                    inv.place_center_y(self.params.width / 4);
                } else {
                    inv.orientation_mut().reflect_vert();
                    inv.place_center_y(3 * self.params.width / 4);
                }

                let src = inv.port("sd_0_0")?.largest_rect(m0)?;
                let dst = inv.port("sd_1_0")?.largest_rect(m0)?;
                let short = src.bbox().union(dst.bbox()).into_rect();
                ctx.draw_rect(m0, short);
                if j == 0 {
                    outs[i] = Some(short);
                }

                for (port, name) in [("sd_0_1", "vss"), ("sd_1_1", "vdd")] {
                    let pwr = inv.port(port)?.largest_rect(m0)?;
                    let viap = ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(pwr, pwr)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw_ref(&via)?;

                    let power_span =
                        Span::from_center_span_gridded(via.brect().center().x, POWER_HEIGHT, GRID);
                    let power_stripe = Rect::from_spans(power_span, stripe_span);
                    let viap = ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(via.layer_bbox(m1), power_stripe)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw(via)?;
                    if i == 0 {
                        ctx.draw_rect(m2, power_stripe);
                        ctx.merge_port(CellPort::with_shape(name, m2, power_stripe));
                    }
                }

                if j == 1 {
                    let dst = inv.port("gate_0")?.largest_rect(m0)?;
                    let jog = SJog::builder()
                        .src(outs[i].unwrap())
                        .dst(dst)
                        .dir(Dir::Horiz)
                        .layer(m0)
                        .width(170)
                        .grid(GRID)
                        .build()
                        .unwrap();
                    ctx.draw(jog)?;

                    let extent = short.right() + 2 * stripe_width + 2 * stripe_space + 40;
                    let m0_conn = Rect::new(
                        short.corner(Corner::LowerLeft),
                        Point::new(extent, short.top()),
                    );
                    ctx.draw_rect(m0, m0_conn);
                    let out_span = Span::with_start_and_length(
                        short.right() + stripe_space + i as i64 * (stripe_width + stripe_space),
                        stripe_width,
                    );
                    let stripe = Rect::from_spans(out_span, stripe_span);
                    ctx.draw_rect(m2, stripe);
                    let name = if i == 0 {
                        arcstr::literal!("outn")
                    } else {
                        arcstr::literal!("outp")
                    };
                    ctx.add_port(CellPort::with_shape(name, m2, stripe));

                    let viap = ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(m0_conn, m0_conn)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw_ref(&via)?;

                    let viap = ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(via.layer_bbox(m1), stripe)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw_ref(&via)?;
                } else {
                    let input = inv.port("gate_0")?.largest_rect(m0)?;
                    let extent = input.left() - 2 * stripe_space - 2 * stripe_width;
                    let m0_conn = Rect::new(Point::new(extent - 40, input.bottom()), input.p1);
                    let in_span = Span::with_start_and_length(
                        extent + i as i64 * (stripe_space + stripe_width),
                        stripe_width,
                    );
                    ctx.draw_rect(m0, m0_conn);
                    let stripe = Rect::from_spans(in_span, stripe_span);
                    ctx.draw_rect(m2, stripe);
                    let name = if i == 0 {
                        arcstr::literal!("inn")
                    } else {
                        arcstr::literal!("inp")
                    };
                    ctx.add_port(CellPort::with_shape(name, m2, stripe));

                    let viap = ViaParams::builder()
                        .layers(m0, m1)
                        .geometry(m0_conn, m0_conn)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw_ref(&via)?;

                    let viap = ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(via.layer_bbox(m1), stripe)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    let via = ctx.instantiate::<Via>(&viap)?;
                    ctx.draw_ref(&via)?;
                }
                ctx.draw(inv)?;
            }
        }

        let vspan = Span::new(0, self.params.width);
        let bounds = Rect::from_spans(ctx.brect().hspan(), vspan);
        ctx.flatten();
        ctx.trim(&bounds);

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::out_gds;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    const PARAMS: DiffBufParams = DiffBufParams {
        lch: 150,
        nw: 1_000,
        pw: 2_000,
        width: 4_800,
    };

    #[test]
    fn test_diff_buf() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_diff_buf");
        ctx.write_layout::<DiffBuf>(&PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
