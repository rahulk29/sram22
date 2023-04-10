use substrate::layout::cell::{CellPort, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignMode;
use substrate::layout::placement::array::{ArrayTiler, ArrayTilerBuilder};
use substrate::layout::placement::tile::LayerBbox;
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::pdk::stdcell::StdCell;

use subgeom::Dir;

use super::ControlLogicReplicaV2;

impl ControlLogicReplicaV2 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;
        let inv = lib.try_cell_named("sky130_fd_sc_hd__inv_2")?;
        let inv = ctx.instantiate::<StdCell>(&inv.id())?;

        let layers = ctx.layers();
        let _m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        let layers = ctx.inner().layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let inv = LayerBbox::new(inv, outline);

        let new_row = || {
            let mut row = ArrayTilerBuilder::new();
            row.mode(AlignMode::ToTheRight).alt_mode(AlignMode::Top);
            row
        };

        let mut rows = ArrayTilerBuilder::new();
        rows.mode(AlignMode::Left).alt_mode(AlignMode::Beneath);

        let mut row0 = new_row();
        row0.push_num(inv.clone(), 24);

        let mut row1 = new_row();
        row1.push_num(inv, 2);

        rows.push(LayerBbox::new(row0.build().generate()?, outline));

        let mut row1 = row1.build().generate()?;
        row1.reflect_vert_anchored();
        rows.push(LayerBbox::new(row1, outline));
        /* for i in 0..23 {
            let y = row0
                .port_map()
                .port(PortId::new("y", i))?
                .largest_rect(m0)?;
            let a = row0
                .port_map()
                .port(PortId::new("a", i + 1))?
                .largest_rect(m0)?;
            let r = Rect::from_spans(Span::new(y.left(), a.right()), a.vspan());
            ctx.draw_rect(m0, r);
        } */

        ctx.draw(rows.build())?;
        let bbox = ctx.bbox();

        let router = GreedyRouter::with_config(GreedyRouterConfig {
            area: bbox.into_rect(),
            layers: vec![
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        ctx.draw(router)?;
        Ok(())
    }
}

#[allow(unused)]
fn draw_row<'a>(
    _ctx: &mut LayoutCtx,
    row: &'a mut ArrayTilerBuilder,
) -> substrate::error::Result<ArrayTiler<'a>> {
    let mut tiler = row.build();
    tiler.expose_ports(
        |mut port: CellPort, i| {
            port.set_id(PortId::new(port.name(), i));
            Some(port)
        },
        PortConflictStrategy::Error,
    )?;
    Ok(tiler)
}
