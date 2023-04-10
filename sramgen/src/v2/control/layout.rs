use substrate::layout::cell::{CellPort, PortConflictStrategy, PortId};
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::align::AlignMode;
use substrate::layout::placement::array::ArrayTilerBuilder;
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

        let layers = ctx.inner().layers();
        let sc_outline = layers.get(Selector::Name("outline"))?;

        let mut row = ArrayTilerBuilder::new();
        row.mode(AlignMode::ToTheRight).alt_mode(AlignMode::Top);
        row.push_num(LayerBbox::new(inv, sc_outline), 24);

        let mut tiler = row.build();
        tiler.expose_ports(
            |mut port: CellPort, i| {
                port.set_id(PortId::new(port.name(), i));
                Some(port)
            },
            PortConflictStrategy::Error,
        )?;
        ctx.draw(row.build())?;

        let bbox = ctx.bbox();

        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        let _router = GreedyRouter::with_config(GreedyRouterConfig {
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
        for i in 0..23 {
            let _y = tiler.port_map().port(PortId::new("Y", i))?;
            let _a = tiler.port_map().port(PortId::new("A", i + 1))?;
        }
        Ok(())
    }
}
