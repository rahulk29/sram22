use serde::Serialize;
use substrate::{
    component::Component,
    layout::{
        cell::{CellPort, PortConflictStrategy},
        placement::{align::AlignMode, array::ArrayTiler},
    },
};

mod layout;
mod schematic;

pub struct ReadMux {
    params: ReadMuxParams,
}

/// ReadMux taps.
pub struct ReadMuxCent {
    params: ReadMuxParams,
}

/// ReadMux end cap.
pub struct ReadMuxEnd {
    params: ReadMuxParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadMuxParams {
    pub length: i64,
    pub width: i64,
    pub mux_ratio: usize,
    pub idx: usize,
}

pub struct TappedReadMux {
    pub params: ReadMuxParams,
}

impl Component for ReadMux {
    type Params = ReadMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("read_mux")
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

impl Component for ReadMuxCent {
    type Params = ReadMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("read_mux_cent")
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

impl Component for ReadMuxEnd {
    type Params = ReadMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("read_mux_end")
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

impl Component for TappedReadMux {
    type Params = ReadMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(TappedReadMux {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_read_mux")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let mut gate = ctx.instantiate::<ReadMux>(&self.params)?;
        ctx.bubble_all_ports(&mut gate);
        ctx.add_instance(gate);
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let params = ReadMuxParams {
            idx: 0,
            ..self.params
        };
        let gate = ctx.instantiate::<ReadMux>(&params)?;
        let tap = ctx.instantiate::<ReadMuxEnd>(&params)?;
        let mut tiler = ArrayTiler::builder()
            .push(tap)
            .push(gate)
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::Bottom)
            .build();
        tiler.expose_ports(
            |port: CellPort, _i| match port.name().as_str() {
                "sel_b" => {
                    if port.id().index() == 0 {
                        Some(port.with_id("sel_b"))
                    } else {
                        None
                    }
                }
                "read_bl" => Some(port.with_id("bl_out")),
                "read_br" => Some(port.with_id("br_out")),
                _ => Some(port),
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

    const READ_MUX_PARAMS: ReadMuxParams = ReadMuxParams {
        length: 150,
        width: 4_000,
        mux_ratio: 4,
        idx: 2,
    };

    #[test]
    fn test_read_mux() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_read_mux");
        ctx.write_layout::<ReadMux>(&READ_MUX_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<ReadMux>(&READ_MUX_PARAMS, out_spice(&work_dir, "schematic"))
            .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<TappedReadMux>(&READ_MUX_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_read_mux_cent() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_read_mux_cent");
        ctx.write_layout::<ReadMuxCent>(&READ_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_read_mux_end() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_read_mux_end");
        ctx.write_layout::<ReadMuxEnd>(&READ_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_rmux_cap() {
        use std::collections::HashMap;

        use substrate::schematic::netlist::NetlistPurpose;

        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_rmux_cap");

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        ctx.write_schematic_to_file_for_purpose::<TappedReadMux>(
            &READ_MUX_PARAMS,
            &pex_path,
            NetlistPurpose::Pex,
        )
        .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<TappedReadMux>(&READ_MUX_PARAMS, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(substrate::verification::pex::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("tapped_read_mux"),
            layout_format: substrate::layout::LayoutFormat::Gds,
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("tapped_read_mux"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let selb_work_dir = work_dir.join("selb_sim");
        let cap_selb = ctx
            .write_simulation::<AcImpedanceTestbench<TappedReadMux>>(
                &AcImpedanceTbParams {
                    fstart: 100.,
                    fstop: 100e6,
                    points: 10,
                    vdd: 1.8,
                    dut: READ_MUX_PARAMS,
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vss,
                    connections: HashMap::from_iter([
                        (arcstr::literal!("sel_b"), vec![AcImpedanceTbNode::Vmeas]),
                        (arcstr::literal!("bl"), vec![AcImpedanceTbNode::Vdd]),
                        (arcstr::literal!("br"), vec![AcImpedanceTbNode::Vdd]),
                        (
                            arcstr::literal!("bl_out"),
                            vec![AcImpedanceTbNode::Floating],
                        ),
                        (
                            arcstr::literal!("br_out"),
                            vec![AcImpedanceTbNode::Floating],
                        ),
                        (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                    ]),
                },
                &selb_work_dir,
            )
            .expect("failed to write simulation");

        println!("Cselb = {}", cap_selb.max_freq_cap(),);
    }
}
