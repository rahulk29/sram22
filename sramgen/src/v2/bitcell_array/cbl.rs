//! Testbench for extracting bitline capacitance.

use substrate::component::Component;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::idc::Idc;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::signal::Signal;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::{Analysis, Save, TranAnalysis};

use super::{SpCellArray, SpCellArrayParams};
use serde::{Deserialize, Serialize};

pub struct BitlineCapTb {
    params: SpCellArrayParams,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitlineCap {
    cbl: f64,
}

const IBL_NANO: i64 = 10;

impl Component for BitlineCapTb {
    type Params = SpCellArrayParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("bitline_cap_testbench")
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);

        let bl = ctx.signal("bl");
        let vdd = ctx.signal("vdd");

        let mut bls = vec![vdd; self.params.cols];
        bls[0] = bl;

        let mut dut = ctx.instantiate::<SpCellArray>(&self.params)?;
        dut.connect_all([("vdd", vdd), ("vss", vss), ("vnb", vss), ("vpb", vdd)]);
        dut.connect("bl", Signal::new(bls));
        dut.connect("br", Signal::repeat(vdd, self.params.cols));
        dut.connect("wl", Signal::repeat(vss, self.params.rows));
        dut.set_name("dut");
        ctx.add_instance(dut);

        let mut vdc = ctx.instantiate::<Vdc>(&SiValue::new(1_800, SiPrefix::Milli))?;
        vdc.connect_all([("p", vdd), ("n", vss)]);
        vdc.set_name("vvdd");
        ctx.add_instance(vdc);

        let mut idc = ctx.instantiate::<Idc>(&SiValue::new(IBL_NANO, SiPrefix::Nano))?;
        idc.connect_all([("p", vss), ("n", bl)]);
        idc.set_name("iin");
        ctx.add_instance(idc);

        ctx.set_spice(".ic v(bl)=0");
        Ok(())
    }
}

impl Testbench for BitlineCapTb {
    type Output = BitlineCap;

    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        ctx.add_analysis(Analysis::Tran(TranAnalysis {
            stop: 6e-6,
            start: 0.0,
            step: 1e-9,
        }))
        .save(Save::All);
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        let data = ctx.output().data[0].tran();
        let sig = &data.data["v(xdut.bl)"];
        let (idx1, v1) = sig
            .values
            .iter()
            .enumerate()
            .filter(|(i, &x)| x > 0.1)
            .next()
            .unwrap();
        let (idx2, v2) = sig
            .values
            .iter()
            .enumerate()
            .filter(|(i, &x)| x > 1.7)
            .next()
            .unwrap();

        let t1 = data.time.values[idx1];
        let t2 = data.time.values[idx2];

        assert!(v2 > v1);
        assert!(idx2 > idx1);
        assert!(t2 > t1);

        let cbl = IBL_NANO as f64 * 1e-9 * (t2 - t1) / (v2 - v1);

        println!("cbl = {:?}", cbl);
        Ok(BitlineCap { cbl })
    }
}

#[cfg(test)]
mod tests {
    use substrate::component::NoParams;
    use substrate::layout::geom::Rect;
    use substrate::layout::layers::selector::Selector;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use crate::v2::bitcell_array::layout::*;
    use crate::v2::guard_ring::{GuardRingParams, GuardRingWrapper, WrapperParams};

    use super::*;

    #[test]
    fn test_bitline_cap_tb() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_bitline_cap_tb");
        let params = SpCellArrayParams {
            rows: 128,
            cols: 8,
            mux_ratio: 4,
        };
        let cap = ctx
            .write_simulation::<BitlineCapTb>(&params, &work_dir)
            .expect("failed to write schematic");
        println!("Cbl = {:?}", cap);
    }
}
