use arcstr::ArcStr;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, NoParams};

pub mod layout;
pub mod schematic;
pub mod testbench;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ControlLogicReplicaV2 {
    params: ControlLogicParams,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ControlLogicParams {
    pub decoder_delay_invs: usize,
    pub wlen_pulse_invs: usize,
    pub pc_set_delay_invs: usize,
    pub wrdrven_delay_invs: usize,
}

impl Component for ControlLogicReplicaV2 {
    type Params = ControlLogicParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        assert_eq!(
            params.decoder_delay_invs % 2,
            0,
            "decoder replica delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wlen_pulse_invs % 2,
            1,
            "wordline pulse delay chain must have an odd number of inverters"
        );
        assert_eq!(
            params.pc_set_delay_invs % 2,
            0,
            "pc set delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wrdrven_delay_invs % 2,
            0,
            "write drive enable delay chain must have an even number of inverters"
        );
        Ok(Self { params: *params })
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
        let n = *params;
        assert!(n >= 1, "inverter chain must have at least one inverter");
        Ok(Self { n })
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

pub struct SvtInvChain {
    n: usize,
}

impl Component for SvtInvChain {
    type Params = usize;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        let n = *params;
        assert!(n >= 1, "inverter chain must have at least one inverter");
        Ok(Self { n })
    }
    fn name(&self) -> arcstr::ArcStr {
        ArcStr::from(format!("svt_inv_chain_{}", self.n))
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

pub struct EdgeDetector {
    invs: usize,
}

impl Component for EdgeDetector {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { invs: 9 })
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

#[cfg(test)]
pub mod test {
    use substrate::component::NoParams;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::{ControlLogicParams, ControlLogicReplicaV2, EdgeDetector, SrLatch};

    const CONTROL_LOGIC_PARAMS: ControlLogicParams = ControlLogicParams {
        decoder_delay_invs: 20,
        wlen_pulse_invs: 11,
        pc_set_delay_invs: 8,
        wrdrven_delay_invs: 2,
    };

    #[test]
    fn test_control_logic_replica_v2() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_control_logic_replica_v2");

        ctx.write_schematic_to_file::<ControlLogicReplicaV2>(
            &CONTROL_LOGIC_PARAMS,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        ctx.write_layout::<ControlLogicReplicaV2>(
            &CONTROL_LOGIC_PARAMS,
            out_gds(&work_dir, "layout"),
        )
        .expect("failed to write layout");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ControlLogicReplicaV2>(&CONTROL_LOGIC_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<ControlLogicReplicaV2>(&CONTROL_LOGIC_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[cfg(feature = "commercial")]
    #[test]
    fn test_control_logic_replica_v2_tb() {
        use crate::blocks::control::testbench::ControlLogicTestbench;

        use super::testbench::tb_params;

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_control_logic_replica_v2_tb");

        ctx.write_simulation_with_corner::<ControlLogicTestbench>(
            &tb_params(1.8),
            &work_dir,
            ctx.corner_db().corner_named("tt").unwrap().clone(),
        )
        .expect("failed to run simulation");
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
