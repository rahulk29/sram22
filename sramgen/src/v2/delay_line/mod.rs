use serde::{Deserialize, Serialize};
use substrate::schematic::circuit::Direction;
use substrate::{component::Component, index::IndexOwned};

use self::transmission::TransmissionGate;
use self::tristate::{TristateBuf, TristateBufParams, TristateInv};

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

pub struct TristateInvDelayLine {
    params: TristateInvDelayLineParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TristateInvDelayLineParams {
    stages: usize,
    inv: PrimitiveGateParams,
    tristate_inv: PrimitiveGateParams,
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
        arcstr::format!("naive_delay_line_{}", self.params.stages)
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

impl Component for TristateInvDelayLine {
    type Params = TristateInvDelayLineParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        assert!(params.stages >= 3);
        Ok(Self { params: *params })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("tristate_inv_delay_line_{}", self.params.stages)
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

        let clk_int_top = ctx.bus("clk_int_top", self.params.stages);
        let clk_int_bot = ctx.bus("clk_int_bot", self.params.stages - 1);

        ctx.instantiate::<Inv>(&self.params.inv)?
            .named("inv_0")
            .with_connections([
                ("din", clk_in),
                ("din_b", clk_int_top.index(0)),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .add_to(ctx);

        ctx.instantiate::<TristateInv>(&self.params.tristate_inv)?
            .named("tristate_inv_mid_0")
            .with_connections([
                ("din", clk_int_top.index(0)),
                ("din_b", clk_out),
                ("en", ctl.index(0)),
                ("en_b", ctl_b.index(0)),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .add_to(ctx);

        for i in 1..self.params.stages {
            ctx.instantiate::<Inv>(&self.params.inv)?
                .named(format!("inv_{i}"))
                .with_connections([
                    ("din", clk_int_top.index(i - 1)),
                    ("din_b", clk_int_top.index(i)),
                    ("vdd", vdd),
                    ("vss", vss),
                ])
                .add_to(ctx);

            ctx.instantiate::<TristateInv>(&self.params.tristate_inv)?
                .named(format!("tristate_inv_mid_{i}"))
                .with_connections([
                    ("din", clk_int_top.index(i)),
                    ("din_b", clk_int_bot.index(i - 1)),
                    ("en", ctl.index(i)),
                    ("en_b", ctl_b.index(i)),
                    ("vdd", vdd),
                    ("vss", vss),
                ])
                .add_to(ctx);

            ctx.instantiate::<TristateInv>(&self.params.tristate_inv)?
                .named(format!("tristate_inv_bot_{i}"))
                .with_connections([
                    ("din", clk_int_bot.index(i - 1)),
                    (
                        "din_b",
                        if i == 1 {
                            clk_out
                        } else {
                            clk_int_bot.index(i - 2)
                        },
                    ),
                    ("en", ctl_b.index(i - 1)),
                    ("en_b", ctl.index(i - 1)),
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
        tristate::TristateBufParams,
        NaiveDelayLine, NaiveDelayLineParams, TristateInvDelayLine, TristateInvDelayLineParams,
    };

    const INV_SIZING: PrimitiveGateParams = PrimitiveGateParams {
        length: 150,
        nwidth: 1_000,
        pwidth: 1_800,
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
        stages: 100,
        inv1: INV_SIZING,
        inv2: INV_SIZING,
        pass: super::PassGateKind::TransmissionGate(TGATE_SIZING),
    };

    const NAIVE_DELAY_LINE_TRISTATE_PARAMS: NaiveDelayLineParams = NaiveDelayLineParams {
        stages: 100,
        inv1: INV_SIZING,
        inv2: INV_SIZING,
        pass: super::PassGateKind::TristateBuf(TRISTATE_SIZING),
    };

    const TRISTATE_INV_DELAY_LINE_PARAMS: TristateInvDelayLineParams = TristateInvDelayLineParams {
        stages: 100,
        inv: INV_SIZING,
        tristate_inv: INV_SIZING,
    };

    const NAIVE_DELAY_LINE_TGATE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::Naive(NAIVE_DELAY_LINE_TGATE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: Some(10e-8),
    };

    const NAIVE_DELAY_LINE_TRISTATE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::Naive(NAIVE_DELAY_LINE_TRISTATE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: Some(10e-8),
    };

    const TRISTATE_INV_DELAY_LINE_TB_PARAMS: DelayLineTbParams = DelayLineTbParams {
        inner: super::tb::DelayLineKind::TristateInv(TRISTATE_INV_DELAY_LINE_PARAMS),
        vdd: 1.8,
        f: 1e9,
        tr: 20e-12,
        ctl_period: 1e-8,
        t_stop: Some(10e-8),
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

    #[test]
    fn test_tristate_inv_delay_line() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tristate_inv_delay_line");
        ctx.write_schematic_to_file::<TristateInvDelayLine>(
            &TRISTATE_INV_DELAY_LINE_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        ctx.write_simulation::<DelayLineTb>(&TRISTATE_INV_DELAY_LINE_TB_PARAMS, work_dir)
            .expect("failed to run simulation");
    }
}
