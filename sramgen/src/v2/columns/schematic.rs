use substrate::component::NoParams;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use crate::v2::buf::DiffBuf;
use crate::v2::macros::{Dff, SenseAmp};
use crate::v2::precharge::Precharge;
use crate::v2::rmux::{ReadMux, ReadMuxParams};
use crate::v2::wmux::{WriteMux, WriteMuxParams};

use super::{ColPeripherals, Column};

impl ColPeripherals {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let cols = self.params.cols;
        let mux_ratio = self.params.mux_ratio();
        let word_length = self.params.word_length();
        let wmask_bits = self.params.wmask_bits();

        let clk = ctx.port("clk", Direction::Input);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let bl = ctx.bus_port("bl", cols, Direction::InOut);
        let br = ctx.bus_port("br", cols, Direction::InOut);
        let bl_dummy = ctx.bus_port("bl_dummy", 2, Direction::InOut);
        let br_dummy = ctx.bus_port("br_dummy", 2, Direction::InOut);
        let pc_b = ctx.port("pc_b", Direction::Input);
        let sel_b = ctx.bus_port("sel_b", mux_ratio, Direction::Input);
        let we = ctx.bus_port("we", mux_ratio, Direction::Input);
        let wmask = ctx.bus_port("wmask", wmask_bits, Direction::Input);
        let din = ctx.bus_port("din", word_length, Direction::Input);
        let dout = ctx.bus_port("dout", word_length, Direction::Output);

        for i in 0..word_length {
            let mut col = ctx.instantiate::<Column>(&self.params)?;
            col.connect_all([
                ("clk", &clk),
                ("vdd", &vdd),
                ("vss", &vss),
                ("bl", &bl.index(i)),
                ("br", &br.index(i)),
                ("pc_b", &pc_b),
                ("sel_b", &sel_b),
                ("we", &we),
                ("wmask", &wmask.index(i / self.params.wmask_granularity)),
                ("din", &din.index(i)),
                ("dout", &dout.index(i)),
            ]);
            ctx.add_instance(col);
        }

        // Add precharges for dummy rows.
        for i in 0..2 {
            let mut pc = ctx.instantiate::<Precharge>(&self.params.pc)?;
            pc.connect_all([
                ("vdd", &vdd),
                ("bl", &bl_dummy.index(i)),
                ("br", &br_dummy.index(i)),
                ("en_b", &pc_b),
            ]);
            pc.set_name(format!("precharge_dummy_{i}"));
            ctx.add_instance(pc);
        }

        Ok(())
    }
}

impl Column {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let clk = ctx.port("clk", Direction::Input);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let bl = ctx.bus_port("bl", self.params.rmux.mux_ratio, Direction::InOut);
        let br = ctx.bus_port("br", self.params.rmux.mux_ratio, Direction::InOut);
        let pc_b = ctx.port("pc_b", Direction::Input);
        let sel_b = ctx.bus_port("sel_b", self.params.rmux.mux_ratio, Direction::Input);
        let we = ctx.port("we", Direction::Input);
        let wmask = ctx.port("wmask", Direction::Input);
        let din = ctx.port("din", Direction::Input);
        let dout = ctx.port("dout", Direction::Output);

        let bl_out = ctx.signal("bl_out");
        let br_out = ctx.signal("br_out");
        let sa_outp = ctx.signal("sa_outp");
        let sa_outn = ctx.signal("sa_outn");
        let diff_buf_outn = ctx.signal("diff_buf_outn");
        let q = ctx.signal("q");
        let q_b = ctx.signal("q_b");

        let mux_ratio = self.params.rmux.mux_ratio;
        let pc = ctx.instantiate::<Precharge>(&self.params.pc)?;

        for i in 0..mux_ratio {
            let bl_i = bl.index(i);
            let br_i = br.index(i);

            let mut pc_i = pc.clone();
            pc_i.connect_all([("vdd", &vdd), ("bl", &bl_i), ("br", &br_i), ("en_b", &pc_b)]);
            pc_i.set_name(format!("precharge_{i}"));
            ctx.add_instance(pc_i);

            let mut rmux = ctx.instantiate::<ReadMux>(&ReadMuxParams {
                idx: i,
                ..self.params.rmux.clone()
            })?;
            rmux.connect_all([
                ("sel_b", &sel_b.index(i)),
                ("bl", &bl_i),
                ("br", &br_i),
                ("bl_out", &bl_out),
                ("br_out", &br_out),
                ("vdd", &vdd),
            ]);
            rmux.set_name(format!("read_mux_{i}"));
            ctx.add_instance(rmux);

            let mut wmux = ctx.instantiate::<WriteMux>(&WriteMuxParams {
                sizing: self.params.wmux,
                idx: i,
            })?;
            wmux.connect_all([
                ("we", &we),
                ("wmask", &wmask),
                ("data", &q),
                ("data_b", &q_b),
                ("bl", &bl_i),
                ("br", &br_i),
                ("vss", &vss),
            ]);
            wmux.set_name(format!("write_mux_{i}"));
            ctx.add_instance(wmux);
        }

        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.connect_all([
            ("clk", &clk),
            ("inn", &br_out),
            ("inp", &bl_out),
            ("outp", &sa_outp),
            ("outn", &sa_outn),
            ("VDD", &vdd),
            ("VSS", &vss),
        ]);
        sa.set_name("sense_amp");
        ctx.add_instance(sa);

        let mut buf = ctx.instantiate::<DiffBuf>(&self.params.buf)?;
        buf.connect_all([
            ("vdd", &vdd),
            ("vss", &vss),
            ("din1", &sa_outp),
            ("din2", &sa_outn),
            ("dout1", &dout),
            ("dout2", &diff_buf_outn),
        ]);
        buf.set_name("buf");
        ctx.add_instance(buf);

        let mut dff = ctx.instantiate::<Dff>(&NoParams)?;
        dff.connect_all([
            ("VDD", &vdd),
            ("GND", &vss),
            ("CLK", &clk),
            ("D", &din),
            ("Q", &q),
            ("Q_N", &q_b),
        ]);
        dff.set_name("dff");
        ctx.add_instance(dff);

        Ok(())
    }
}
