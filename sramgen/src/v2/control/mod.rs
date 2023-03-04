use std::path::PathBuf;

use arcstr::ArcStr;
use codegen::hard_macro;

use substrate::component::{Component, NoParams, View};
use substrate::data::SubstrateCtx;
use substrate::index::IndexOwned;
use subgeom::Corner;
use substrate::layout::placement::align::AlignMode;
use substrate::layout::placement::array::ArrayTiler;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::schematic::circuit::Direction;

use crate::bus_bit;
use crate::tech::{external_gds_path, external_spice_path};

use super::macros::Dff;
fn path(_ctx: &SubstrateCtx, name: &str, view: View) -> Option<PathBuf> {
    match view {
        View::Layout => Some(external_gds_path().join(format!("{name}.gds"))),
        View::Schematic => Some(external_spice_path().join(format!("{name}.spice"))),
        _ => None,
    }
}

#[hard_macro(
    name = "sramgen_control_logic_replica_v1",
    pdk = "sky130-open",
    path_fn = "path",
    gds_cell_name = "sramgen_control_logic_replica_v1",
    spice_subckt_name = "sramgen_control_logic_replica_v1"
)]
pub struct ControlLogicReplicaV1;

pub struct DffArray {
    n: usize,
}

impl Component for DffArray {
    type Params = usize;
    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self { n: *params })
    }
    fn name(&self) -> ArcStr {
        arcstr::format!("dff_array_{}", self.n)
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let n = self.n;
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let clk = ctx.port("clk", Direction::Input);
        let d = ctx.bus_port("d", n, Direction::Input);
        let q = ctx.bus_port("q", n, Direction::Output);
        let qn = ctx.bus_port("qn", n, Direction::Output);

        for i in 0..self.n {
            ctx.instantiate::<Dff>(&NoParams)?
                .with_connections([
                    ("VDD", vdd),
                    ("GND", vss),
                    ("CLK", clk),
                    ("D", d.index(i)),
                    ("Q", q.index(i)),
                    ("Q_N", qn.index(i)),
                ])
                .named(format!("dff_{i}"))
                .add_to(ctx);
        }

        Ok(())
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let mut dff = ctx.instantiate::<Dff>(&NoParams)?;
        let tiler = ArrayTiler::builder()
            .mode(AlignMode::ToTheRight)
            .push_num(dff.clone(), self.n)
            .build();

        for i in 0..self.n {
            dff.place(Corner::LowerLeft, tiler.cell(i).corner(Corner::LowerLeft));
            for port in ["q", "qn", "clk", "d"] {
                ctx.add_port(dff.port(port)?.into_cell_port().named(bus_bit(port, i)));
            }
        }
        ctx.draw(tiler)?;
        Ok(())
    }
}
