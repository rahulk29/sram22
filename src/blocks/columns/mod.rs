//! Column peripheral circuitry.

use substrate::component::Component;
use substrate::layout::context::LayoutCtx;
use substrate::schematic::context::SchematicCtx;

use super::buf::DiffBufParams;
use super::precharge::PrechargeParams;
use super::rmux::ReadMuxParams;
use super::tgatemux::TGateMuxParams;
use super::wmux::WriteMuxSizing;
use super::wrdriver::WriteDriverParams;
use serde::Serialize;

pub mod layout;
pub mod schematic;

#[derive(Debug, Clone, Serialize)]
pub struct ColParams {
    pub pc: PrechargeParams,
    pub mux: TGateMuxParams,
    pub wrdriver: WriteDriverParams,
    pub buf: DiffBufParams,
    pub cols: usize,
    pub include_wmask: bool,
    pub wmask_granularity: usize,
}

impl ColParams {
    fn mux_ratio(&self) -> usize {
        self.mux.mux_ratio
    }

    fn word_length(&self) -> usize {
        self.cols / self.mux_ratio()
    }

    fn wmask_bits(&self) -> usize {
        self.word_length() / self.wmask_granularity
    }
}

pub struct ColPeripherals {
    params: ColParams,
}

pub struct Column {
    params: ColParams,
}

impl Component for ColPeripherals {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("col_peripherals")
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for Column {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column")
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

#[cfg(test)]
mod tests {

    use arcstr::ArcStr;
    use subgeom::bbox::{Bbox, BoundBox};
    use substrate::layout::cell::{CellPort, Port, PortId};
    use substrate::layout::layers::selector::Selector;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::layout::{ColCentParams, ColumnCent};
    use super::*;

    const WRITE_DRIVER_PARAMS: WriteDriverParams = WriteDriverParams {
        length: 150,
        pwidth_driver: 3_000,
        nwidth_driver: 3_000,
        pwidth_logic: 3_000,
        nwidth_logic: 3_000,
    };
    const MUX_PARAMS: TGateMuxParams = TGateMuxParams {
        length: 150,
        pwidth: 3_000,
        nwidth: 3_000,
        mux_ratio: 4,
        idx: 2,
    };
    const PRECHARGE_PARAMS: PrechargeParams = PrechargeParams {
        length: 150,
        pull_up_width: 2_000,
        equalizer_width: 1_200,
    };
    const DIFF_BUF_PARAMS: DiffBufParams = DiffBufParams {
        width: 4_800,
        nw: 1_200,
        pw: 2_000,
        lch: 150,
    };

    const COL_WMASK_PARAMS: ColParams = ColParams {
        pc: PRECHARGE_PARAMS,
        wrdriver: WRITE_DRIVER_PARAMS,
        mux: MUX_PARAMS,
        buf: DIFF_BUF_PARAMS,
        cols: 16,
        include_wmask: true,
        wmask_granularity: 2,
    };

    const COL_PARAMS: ColParams = ColParams {
        pc: PRECHARGE_PARAMS,
        wrdriver: WRITE_DRIVER_PARAMS,
        mux: MUX_PARAMS,
        buf: DIFF_BUF_PARAMS,
        cols: 128,
        include_wmask: false,
        wmask_granularity: 8,
    };

    struct ColPeripheralsLvs {
        params: ColParams,
    }

    impl Component for ColPeripheralsLvs {
        type Params = ColParams;

        fn new(
            params: &Self::Params,
            _ctx: &substrate::data::SubstrateCtx,
        ) -> substrate::error::Result<Self> {
            Ok(Self {
                params: params.clone(),
            })
        }

        fn name(&self) -> ArcStr {
            arcstr::literal!("col_peripherals_lvs")
        }

        fn schematic(
            &self,
            ctx: &mut substrate::schematic::context::SchematicCtx,
        ) -> substrate::error::Result<()> {
            let mut cols = ctx.instantiate::<ColPeripherals>(&self.params)?;
            ctx.bubble_all_ports(&mut cols);
            ctx.add_instance(cols);
            Ok(())
        }

        fn layout(
            &self,
            ctx: &mut substrate::layout::context::LayoutCtx,
        ) -> substrate::error::Result<()> {
            let m2 = ctx.layers().get(Selector::Metal(2))?;
            let cols = ctx.instantiate::<ColPeripherals>(&self.params)?;

            for i in 0..2 {
                let clk0 = cols.port(PortId::new("clk", i))?;

                let mut clk0_brect = Bbox::empty();
                for shape in clk0.shapes(m2) {
                    clk0_brect = clk0_brect.union(shape.bbox());
                }

                ctx.draw_rect(m2, clk0_brect.into_rect());
                ctx.merge_port(CellPort::with_shape("clk", m2, clk0_brect.into_rect()));
            }
            ctx.add_ports(cols.ports().filter_map(|port| match port.name().as_str() {
                "clk" => None,
                _ => Some(port),
            }))?;
            ctx.draw(cols)?;

            Ok(())
        }
    }

    #[test]
    fn test_col_peripherals() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_col_peripherals");
        ctx.write_layout::<ColPeripherals>(&COL_WMASK_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<ColPeripherals>(
            &COL_WMASK_PARAMS,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ColPeripherals>(&COL_WMASK_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
        }
    }

    #[test]
    fn test_col_peripherals_lvs() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_col_peripherals_lvs");
        ctx.write_layout::<ColPeripheralsLvs>(&COL_WMASK_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<ColPeripheralsLvs>(
            &COL_WMASK_PARAMS,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<ColPeripheralsLvs>(&COL_WMASK_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_column_wmask_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_wmask_4");
        ctx.write_layout::<Column>(&COL_WMASK_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_column_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_4");
        ctx.write_layout::<Column>(&COL_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        ctx.write_schematic_to_file::<Column>(&COL_PARAMS, out_spice(work_dir, "schematic"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_column_cent_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_cent_4");
        ctx.write_layout::<ColumnCent>(
            &ColCentParams {
                col: COL_WMASK_PARAMS,
                end: false,
                cut_wmask: false,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }

    #[test]
    fn test_column_end_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_end_4");
        ctx.write_layout::<ColumnCent>(
            &ColCentParams {
                col: COL_WMASK_PARAMS,
                end: true,
                cut_wmask: true,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }
}
