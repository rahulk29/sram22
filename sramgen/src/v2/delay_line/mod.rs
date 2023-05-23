use serde::{Deserialize, Serialize};
use substrate::schematic::circuit::Direction;
use substrate::{component::Component, index::IndexOwned};

use self::transmission::TransmissionGate;

use super::gate::{Inv, PrimitiveGateParams};

pub mod tb;
pub mod transmission;

pub struct DelayLine {
    params: DelayLineParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelayLineParams {
    stages: usize,
    inv1: PrimitiveGateParams,
    inv2: PrimitiveGateParams,
    pass: PrimitiveGateParams,
}

impl Component for DelayLine {
    type Params = DelayLineParams;

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

            ctx.instantiate::<TransmissionGate>(&self.params.inv2)?
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{paths::out_spice, setup_ctx, tests::test_work_dir, v2::gate::PrimitiveGateParams};

    use super::{
        tb::{DelayLineTb, DelayLineTbParams},
        DelayLine, DelayLineParams,
    };

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 5_000,
        pwidth: 9_000,
    };

    const PASS_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 500,
        pwidth: 900,
    };

    const DELAY_LINE_PARAMS: DelayLineParams = DelayLineParams {
        stages: 10,
        inv1: INV_SIZING,
        inv2: INV_SIZING,
        pass: PASS_SIZING,
    };

    const DELAY_LINE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: DELAY_LINE_PARAMS,
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: None,
    };

    #[test]
    fn test_delay_line() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_delay_line");
        ctx.write_schematic_to_file::<DelayLine>(
            &DELAY_LINE_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        ctx.write_simulation::<DelayLineTb>(&DELAY_LINE_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }
}
