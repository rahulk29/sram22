use serde::{Deserialize, Serialize};
use substrate::schematic::circuit::Direction;
use substrate::{component::Component, index::IndexOwned};

use self::transmission::TransmissionGate;
use self::tristate::{TristateBuf, TristateBufParams};

use super::gate::{Inv, PrimitiveGateParams};

pub mod tb;
pub mod transmission;
pub mod tristate;

pub struct NaiveDelayLine {
    params: NaiveDelayLineParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PassGateKind {
    TransmissionGate(PrimitiveGateParams),
    TristateBuf(TristateBufParams),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NaiveDelayLineParams {
    stages: usize,
    inv1: PrimitiveGateParams,
    inv2: PrimitiveGateParams,
    pass: PassGateKind,
}

impl Component for NaiveDelayLine {
    type Params = NaiveDelayLineParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        assert!(params.stages >= 3);
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("delay_line_{}", self.params.stages)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let clk_in = ctx.port("clk_in", Direction::Input);
        let clk_out = ctx.port("clk_out", Direction::Output);
        let ctl = ctx.bus_port("ctl", self.params.stages, Direction::Input);
        let ctl_b = ctx.bus_port("ctl_b", self.params.stages, Direction::Input);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);

        let clk_int = ctx.bus("clk_int", self.params.stages);
        let buf_int = ctx.bus("buf_int", self.params.stages);

        for i in 0..self.params.stages {
            let stage_in = if i == 0 { clk_in } else { clk_int.index(i - 1) };
            let stage_out = clk_int.index(i);

            for (j, (input, output, params)) in [
                (stage_in, buf_int.index(i), &self.params.inv1),
                (buf_int.index(i), stage_out, &self.params.inv2),
            ]
            .iter()
            .enumerate()
            {
                ctx.instantiate::<Inv>(*params)?
                    .named(format!("buf_inv_{i}_{j}"))
                    .with_connections([
                        ("din", input),
                        ("din_b", output),
                        ("vdd", &vdd),
                        ("vss", &vss),
                    ])
                    .add_to(ctx);
            }

            match self.params.pass {
                PassGateKind::TransmissionGate(params) => {
                    ctx.instantiate::<TransmissionGate>(&params)?
                        .named(format!("pass_{i}"))
                        .with_connections([
                            ("din", stage_out),
                            ("dout", clk_out),
                            ("en", ctl.index(i)),
                            ("en_b", ctl_b.index(i)),
                            ("vdd", vdd),
                            ("vss", vss),
                        ])
                        .add_to(ctx);
                }
                PassGateKind::TristateBuf(params) => {
                    ctx.instantiate::<TristateBuf>(&params)?
                        .named(format!("pass_{i}"))
                        .with_connections([
                            ("din", stage_out),
                            ("dout", clk_out),
                            ("en", ctl.index(i)),
                            ("en_b", ctl_b.index(i)),
                            ("vdd", vdd),
                            ("vss", vss),
                        ])
                        .add_to(ctx);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{paths::out_spice, setup_ctx, tests::test_work_dir, v2::gate::PrimitiveGateParams};

    use super::{
        tb::{DelayLineTb, DelayLineTbParams},
        tristate::TristateBufParams,
        NaiveDelayLine, NaiveDelayLineParams,
    };

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 5_000,
        pwidth: 9_000,
    };

    const TGATE_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 500,
        pwidth: 900,
    };

    const TRISTATE_SIZING: TristateBufParams = TristateBufParams {
        inv1: INV_SIZING,
        inv2: INV_SIZING,
    };

    const NAIVE_DELAY_LINE_TGATE_PARAMS: NaiveDelayLineParams = NaiveDelayLineParams {
        stages: 10,
        inv1: INV_SIZING,
        inv2: INV_SIZING,
        pass: super::PassGateKind::TransmissionGate(TGATE_SIZING),
    };

    const NAIVE_DELAY_LINE_TRISTATE_PARAMS: NaiveDelayLineParams = NaiveDelayLineParams {
        stages: 10,
        inv1: INV_SIZING,
        inv2: INV_SIZING,
        pass: super::PassGateKind::TristateBuf(TRISTATE_SIZING),
    };

    const NAIVE_DELAY_LINE_TGATE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::Naive(NAIVE_DELAY_LINE_TGATE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: None,
    };

    const NAIVE_DELAY_LINE_TRISTATE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::Naive(NAIVE_DELAY_LINE_TRISTATE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: None,
    };

    #[test]
    fn test_naive_delay_line_tgate() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_naive_delay_line_tgate");
        ctx.write_schematic_to_file::<NaiveDelayLine>(
            &NAIVE_DELAY_LINE_TGATE_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        ctx.write_simulation::<DelayLineTb>(&NAIVE_DELAY_LINE_TGATE_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }

    #[test]
    fn test_naive_delay_line_tristate() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_naive_delay_line_tristate");
        ctx.write_schematic_to_file::<NaiveDelayLine>(
            &NAIVE_DELAY_LINE_TRISTATE_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        ctx.write_simulation::<DelayLineTb>(&NAIVE_DELAY_LINE_TRISTATE_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }
}
