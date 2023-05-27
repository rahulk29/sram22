use arcstr::ArcStr;

use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::index::IndexOwned;
use substrate::layout::cell::CellPort;
use substrate::layout::placement::align::AlignMode;
use substrate::layout::placement::array::ArrayTiler;
use substrate::schematic::circuit::Direction;

use super::macros::Dff;

pub mod layout;
pub mod schematic;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum ControlLogicKind {
    Standard,
    Test,
}

pub struct ControlLogicReplicaV2(ControlLogicKind);

impl Component for ControlLogicReplicaV2 {
    type Params = ControlLogicKind;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self(*params))
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("control_logic_replica_v2")
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

pub struct SrLatch;

impl Component for SrLatch {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sr_latch")
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

pub struct InvChain {
    n: usize,
}

impl Component for InvChain {
    type Params = usize;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { n: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        ArcStr::from(format!("inv_chain_{}", self.n))
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

pub struct EdgeDetector;

impl Component for EdgeDetector {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("edge_detector")
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

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
        let dff = ctx.instantiate::<Dff>(&NoParams)?;
        let mut tiler = ArrayTiler::builder()
            .mode(AlignMode::ToTheRight)
            .push_num(dff, self.n)
            .build();

        tiler.expose_ports(
            |port: CellPort, i| {
                if ["vdd", "vss"].contains(&port.name().as_ref()) {
                    Some(port)
                } else {
                    let port = port.with_index(i);
                    Some(port)
                }
            },
            substrate::layout::cell::PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();

        ctx.draw(tiler)?;
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use arcstr::ArcStr;
    use subgeom::bbox::{Bbox, BoundBox};
    use subgeom::{Dir, Point, Rect};
    use substrate::component::{Component, NoParams};
    use substrate::layout::cell::Port;
    use substrate::layout::elements::via::{Via, ViaParams};
    use substrate::layout::layers::selector::Selector;
    use substrate::layout::layers::LayerBoundBox;
    use substrate::layout::placement::align::{AlignMode, AlignRect};

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::{ControlLogicKind, ControlLogicReplicaV2, EdgeDetector, SrLatch};

    struct ControlLogicReplicaV2Lvs(ControlLogicKind);

    impl Component for ControlLogicReplicaV2Lvs {
        type Params = ControlLogicKind;

        fn new(
            params: &Self::Params,
            _ctx: &substrate::data::SubstrateCtx,
        ) -> substrate::error::Result<Self> {
            Ok(Self(*params))
        }

        fn name(&self) -> ArcStr {
            arcstr::literal!("control_logic_replica_v2_lvs")
        }

        fn schematic(
            &self,
            ctx: &mut substrate::schematic::context::SchematicCtx,
        ) -> substrate::error::Result<()> {
            let mut array = ctx.instantiate::<ControlLogicReplicaV2>(&self.0)?;
            ctx.bubble_all_ports(&mut array);
            ctx.add_instance(array);
            Ok(())
        }

        fn layout(
            &self,
            ctx: &mut substrate::layout::context::LayoutCtx,
        ) -> substrate::error::Result<()> {
            let layers = ctx.layers();
            let m1 = layers.get(Selector::Metal(1))?;
            let m2 = layers.get(Selector::Metal(2))?;

            let control = ctx.instantiate::<ControlLogicReplicaV2>(&self.0)?;

            let via = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(
                        Rect::from_point(Point::zero()),
                        Rect::from_point(Point::zero()),
                    )
                    .bot_extension(Dir::Vert)
                    .top_extension(Dir::Vert)
                    .build(),
            )?;

            let mut rect = Bbox::empty();
            for shape in control.port("vdd")?.shapes(m1) {
                let mut via = via.clone();
                via.align_centers(shape.brect());
                via.align_left(shape.brect());
                rect = rect.union(via.layer_bbox(m2));
                ctx.draw(via)?;
            }
            ctx.draw_rect(m2, rect.into_rect());

            let mut rect = Bbox::empty();
            for shape in control.port("vss")?.shapes(m1) {
                let mut via = via.clone();
                via.align_centers(shape.brect());
                via.align(AlignMode::Left, shape.brect(), 400);
                rect = rect.union(via.layer_bbox(m2));
                ctx.draw(via)?;
            }
            ctx.draw_rect(m2, rect.into_rect());

            ctx.add_ports(control.ports())?;
            ctx.draw(control)?;

            Ok(())
        }
    }

    #[test]
    fn test_control_logic_replica_v2() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_control_logic_replica_v2");

        ctx.write_schematic_to_file::<ControlLogicReplicaV2>(
            &ControlLogicKind::Standard,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        ctx.write_layout::<ControlLogicReplicaV2>(
            &ControlLogicKind::Standard,
            out_gds(&work_dir, "layout"),
        )
        .expect("failed to write layout");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ControlLogicReplicaV2>(&ControlLogicKind::Standard, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<ControlLogicReplicaV2>(&ControlLogicKind::Standard, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_control_logic_replica_v2_test() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_control_logic_replica_v2_test");

        ctx.write_schematic_to_file::<ControlLogicReplicaV2>(
            &ControlLogicKind::Test,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        ctx.write_layout::<ControlLogicReplicaV2>(
            &ControlLogicKind::Test,
            out_gds(&work_dir, "layout"),
        )
        .expect("failed to write layout");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ControlLogicReplicaV2>(&ControlLogicKind::Test, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<ControlLogicReplicaV2>(&ControlLogicKind::Test, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_control_logic_replica_v2_lvs() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_control_logic_replica_v2_lvs");

        ctx.write_schematic_to_file::<ControlLogicReplicaV2Lvs>(
            &ControlLogicKind::Standard,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        ctx.write_layout::<ControlLogicReplicaV2Lvs>(
            &ControlLogicKind::Standard,
            out_gds(&work_dir, "layout"),
        )
        .expect("failed to write layout");

        #[cfg(feature = "commercial")]
        {
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<ControlLogicReplicaV2Lvs>(&ControlLogicKind::Standard, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_sr_latch() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sr_latch");

        ctx.write_layout::<SrLatch>(&NoParams, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_edge_detector() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_edge_detector");

        ctx.write_layout::<EdgeDetector>(&NoParams, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
