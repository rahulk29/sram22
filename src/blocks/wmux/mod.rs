use serde::{Deserialize, Serialize};
use substrate::component::Component;
use substrate::layout::cell::{CellPort, PortConflictStrategy, PortId};
use substrate::layout::placement::align::AlignMode;
use substrate::layout::placement::array::ArrayTiler;

mod layout;
mod schematic;

pub struct WriteMux {
    params: WriteMuxParams,
}

/// WriteMux taps.
pub struct WriteMuxCent {
    params: WriteMuxCentParams,
}

/// WriteMux end cap.
pub struct WriteMuxEnd {
    params: WriteMuxEndParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WriteMuxSizing {
    pub length: i64,
    pub mux_width: i64,
    pub mux_ratio: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteMuxParams {
    pub sizing: WriteMuxSizing,
    pub idx: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteMuxCentParams {
    pub sizing: WriteMuxSizing,
    /// Whether to cut the data line between adjacent muxes.
    pub cut_data: bool,
    /// Whether to cut the wmask line between adjacent muxes.
    pub cut_wmask: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteMuxEndParams {
    pub sizing: WriteMuxSizing,
}

pub struct TappedWriteMux {
    pub sizing: WriteMuxSizing,
}

impl WriteMuxCentParams {
    pub(crate) fn for_wmux(&self) -> WriteMuxParams {
        WriteMuxParams {
            sizing: self.sizing,
            idx: 0,
        }
    }
}

impl WriteMuxEndParams {
    pub(crate) fn for_wmux_cent(&self) -> WriteMuxCentParams {
        WriteMuxCentParams {
            sizing: self.sizing,
            cut_data: true,
            cut_wmask: true,
        }
    }
}

impl Component for WriteMux {
    type Params = WriteMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_mux")
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

impl Component for WriteMuxCent {
    type Params = WriteMuxCentParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_mux_cent")
    }

    fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for WriteMuxEnd {
    type Params = WriteMuxEndParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_mux_end")
    }

    fn schematic(
        &self,
        _ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for TappedWriteMux {
    type Params = WriteMuxSizing;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(TappedWriteMux { sizing: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_write_mux")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let mut gate = ctx.instantiate::<WriteMux>(&WriteMuxParams {
            sizing: self.sizing,
            idx: 0,
        })?;
        ctx.bubble_all_ports(&mut gate);
        ctx.add_instance(gate);
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let gate = ctx.instantiate::<WriteMux>(&WriteMuxParams {
            sizing: self.sizing,
            idx: 0,
        })?;
        let tap = ctx.instantiate::<WriteMuxEnd>(&WriteMuxEndParams {
            sizing: self.sizing,
        })?;
        let mut tiler = ArrayTiler::builder()
            .push(tap)
            .push(gate)
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::Bottom)
            .build();
        tiler.expose_ports(
            |port: CellPort, _i| {
                if port.id() == &PortId::new("we", 0) {
                    Some(port.with_id("we"))
                } else if port.name() == "we" {
                    None
                } else {
                    Some(port)
                }
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();

        ctx.draw_ref(&tiler)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    const WRITE_MUX_SIZING: WriteMuxSizing = WriteMuxSizing {
        length: 150,
        mux_width: 2_000,
        mux_ratio: 4,
    };

    const WRITE_MUX_PARAMS: WriteMuxParams = WriteMuxParams {
        sizing: WRITE_MUX_SIZING,
        idx: 2,
    };
    const WRITE_MUX_CENT_PARAMS: WriteMuxCentParams = WriteMuxCentParams {
        sizing: WRITE_MUX_SIZING,
        cut_data: true,
        cut_wmask: false,
    };
    const WRITE_MUX_END_PARAMS: WriteMuxEndParams = WriteMuxEndParams {
        sizing: WRITE_MUX_SIZING,
    };

    #[test]
    fn test_write_mux() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux");
        ctx.write_layout::<WriteMux>(&WRITE_MUX_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<WriteMux>(
            &WRITE_MUX_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<TappedWriteMux>(&WRITE_MUX_SIZING, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_write_mux_cent() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux_cent");
        ctx.write_layout::<WriteMuxCent>(&WRITE_MUX_CENT_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_write_mux_end() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_mux_end");
        ctx.write_layout::<WriteMuxEnd>(&WRITE_MUX_END_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_wmux_cap() {
        use std::collections::HashMap;

        use substrate::schematic::netlist::NetlistPurpose;

        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_wmux_cap");
        let params = WriteMuxSizing {
            length: 150,
            mux_width: 8_800,
            mux_ratio: 8,
        };

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<TappedWriteMux>(
            &params,
            &pex_path,
            NetlistPurpose::Pex,
        )
        .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<TappedWriteMux>(&params, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("tapped_write_mux"),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("tapped_write_mux"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let we_work_dir = work_dir.join("we_sim");
        let cap_we = ctx
            .write_simulation::<AcImpedanceTestbench<TappedWriteMux>>(
                &AcImpedanceTbParams {
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    vdd: 1.8,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("we"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("wmask"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("data"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("data_b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("bl"), vec![AcImpedanceTbNode::Floating]),
                        (arcstr::literal!("br"), vec![AcImpedanceTbNode::Floating]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                    ]),
                },
                &we_work_dir,
            )
            .expect("failed to write simulation");

        let wmask_work_dir = work_dir.join("wmask_sim");
        let cap_wmask = ctx
            .write_simulation::<AcImpedanceTestbench<TappedWriteMux>>(
                &AcImpedanceTbParams {
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    vdd: 1.8,
                    dut: params,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("we"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("wmask"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("data"), vec![AcImpedanceTbNode::Vss]),
                        (arcstr::literal!("data_b"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("bl"), vec![AcImpedanceTbNode::Floating]),
                        (arcstr::literal!("br"), vec![AcImpedanceTbNode::Floating]),
                        (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                    ]),
                },
                &wmask_work_dir,
            )
            .expect("failed to write simulation");

        println!(
            "Cwe = {}, Cwmask = {}",
            cap_we.max_freq_cap(),
            cap_wmask.max_freq_cap()
        );
    }
}
