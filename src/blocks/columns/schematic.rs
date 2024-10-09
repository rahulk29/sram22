use std::collections::HashMap;
use subgeom::Dir;
use substrate::component::NoParams;
use substrate::error::Result;
use substrate::index::IndexOwned;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use crate::blocks::buf::DiffBuf;
use crate::blocks::decoder::DecoderStage;
use crate::blocks::macros::SenseAmp;
use crate::blocks::precharge::Precharge;
use crate::blocks::tgatemux::TGateMux;
use crate::blocks::wrdriver::WriteDriver;

use super::layout::DffArray;
use super::{
    ColPeripherals, Column, ColumnDesignScript, ColumnsPhysicalDesign, ColumnsPhysicalDesignScript,
};

impl ColPeripherals {
    pub fn io(&self) -> HashMap<&'static str, usize> {
        HashMap::from([
            ("clk", 1),
            ("reset_b", 1),
            ("vdd", 1),
            ("vss", 1),
            ("sense_en", 1),
            ("bl", self.params.cols),
            ("br", self.params.cols),
            ("pc_b", 1),
            ("sel", self.params.mux_ratio()),
            ("sel_b", self.params.mux_ratio()),
            ("we", 1),
            ("wmask", self.params.wmask_bits()),
            ("din", self.params.word_length()),
            ("dout", self.params.word_length()),
        ])
    }

    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let cols = self.params.cols;
        let mux_ratio = self.params.mux_ratio();
        let word_length = self.params.word_length();
        let wmask_bits = self.params.wmask_bits();

        let clk = ctx.port("clk", Direction::Input);
        let reset_b = ctx.port("reset_b", Direction::Input);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let sense_en = ctx.port("sense_en", Direction::Input);
        let bl = ctx.bus_port("bl", cols, Direction::InOut);
        let br = ctx.bus_port("br", cols, Direction::InOut);
        let pc_b = ctx.port("pc_b", Direction::Input);
        let sel = ctx.bus_port("sel", mux_ratio, Direction::Input);
        let sel_b = ctx.bus_port("sel_b", mux_ratio, Direction::Input);
        let we = ctx.port("we", Direction::Input);
        let wmask = ctx.bus_port("wmask", wmask_bits, Direction::Input);
        let din = ctx.bus_port("din", word_length, Direction::Input);
        let dout = ctx.bus_port("dout", word_length, Direction::Output);

        let wmask_in = ctx.bus("wmask_in", wmask_bits);
        let wmask_in_b = ctx.bus("wmask_in_b", wmask_bits);
        let we_i = ctx.bus("we_i", wmask_bits);
        let we_ib = ctx.bus("we_ib", wmask_bits);
        let [dummy_bl, dummy_br, dummy_bl_noconn, dummy_br_noconn] =
            ctx.signals(["dummy_bl", "dummy_br", "dummy_bl_noconn", "dummy_br_noconn"]);

        ctx.instantiate::<DffArray>(&wmask_bits)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("clk", clk),
                ("rb", reset_b),
                ("d", wmask),
                ("q", wmask_in),
                ("qn", wmask_in_b),
            ])
            .named("wmask_dffs")
            .add_to(ctx);

        let ColumnsPhysicalDesign { nand, .. } = &*ctx
            .inner()
            .run_script::<ColumnsPhysicalDesignScript>(&self.params)?;

        for i in 0..wmask_bits {
            ctx.instantiate::<DecoderStage>(&nand)?
                .with_connections([
                    ("predecode_0_0", we),
                    ("predecode_1_0", wmask_in.index(i)),
                    ("y", we_i.index(i)),
                    ("y_b", we_ib.index(i)),
                    ("vdd", vdd),
                    ("vss", vss),
                ])
                .named(arcstr::format!("wmask_and_{i}"))
                .add_to(ctx);
        }

        for i in 0..word_length {
            let range = i * mux_ratio..(i + 1) * mux_ratio;
            ctx.instantiate::<Column>(&self.params)?
                .with_connections([
                    ("clk", &clk),
                    ("reset_b", &reset_b),
                    ("vdd", &vdd),
                    ("vss", &vss),
                    ("pc_b", &pc_b),
                    ("sel", &sel),
                    ("sel_b", &sel_b),
                    ("bl", &bl.index(range.clone())),
                    ("br", &br.index(range)),
                    ("we", &we_i.index(i / self.params.wmask_granularity)),
                    ("we_b", &we_ib.index(i / self.params.wmask_granularity)),
                    ("din", &din.index(i)),
                    ("dout", &dout.index(i)),
                    ("sense_en", &sense_en),
                ])
                .named(arcstr::format!("col_group_{i}"))
                .add_to(ctx);
        }

        ctx.instantiate::<Precharge>(&self.params.pc)?
            .with_connections([
                ("vdd", vdd),
                ("bl", dummy_bl),
                ("br", dummy_br),
                ("en_b", pc_b),
            ])
            .named("dummy_precharge")
            .add_to(ctx);

        ctx.instantiate::<Precharge>(&self.params.pc)?
            .with_connections([
                ("vdd", vdd),
                ("bl", dummy_bl_noconn),
                ("br", dummy_br_noconn),
                ("en_b", pc_b),
            ])
            .named("dummy_precharge_noconn")
            .add_to(ctx);

        Ok(())
    }
}

impl Column {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        let clk = ctx.port("clk", Direction::Input);
        let reset_b = ctx.port("reset_b", Direction::Input);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let bl = ctx.bus_port("bl", self.params.mux_ratio(), Direction::InOut);
        let br = ctx.bus_port("br", self.params.mux_ratio(), Direction::InOut);
        let pc_b = ctx.port("pc_b", Direction::Input);
        let sel = ctx.bus_port("sel", self.params.mux_ratio(), Direction::Input);
        let sel_b = ctx.bus_port("sel_b", self.params.mux_ratio(), Direction::Input);
        let we = ctx.port("we", Direction::Input);
        let we_b = ctx.port("we_b", Direction::Input);
        let din = ctx.port("din", Direction::Input);
        let dout = ctx.port("dout", Direction::Output);
        let sense_en = ctx.port("sense_en", Direction::Input);

        let bl_out = ctx.signal("bl_out");
        let br_out = ctx.signal("br_out");
        let sa_outp = ctx.signal("sa_outp");
        let sa_outn = ctx.signal("sa_outn");
        let diff_buf_outn = ctx.signal("diff_buf_outn");
        let q = ctx.signal("q");
        let q_b = ctx.signal("q_b");

        let mux_ratio = self.params.mux_ratio();
        let pc = ctx.instantiate::<Precharge>(&self.params.pc)?;

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let dfrtp = lib.try_cell_named("sky130_fd_sc_hs__dfrbp_2")?;

        for i in 0..mux_ratio {
            let bl_i = bl.index(i);
            let br_i = br.index(i);

            let mut pc_i = pc.clone();
            pc_i.connect_all([("vdd", &vdd), ("bl", &bl_i), ("br", &br_i), ("en_b", &pc_b)]);
            pc_i.set_name(format!("precharge_{i}"));
            ctx.add_instance(pc_i);

            let mut mux = ctx.instantiate::<TGateMux>(&self.params.mux)?;
            mux.connect_all([
                ("sel_b", &sel_b.index(i)),
                ("sel", &sel.index(i)),
                ("bl", &bl_i),
                ("br", &br_i),
                ("bl_out", &bl_out),
                ("br_out", &br_out),
                ("vdd", &vdd),
                ("vss", &vss),
            ]);
            mux.set_name(format!("mux_{i}"));
            ctx.add_instance(mux);
        }

        let mut wrdrv = ctx.instantiate::<WriteDriver>(&self.params.wrdriver)?;
        wrdrv.connect_all([
            ("en", &we),
            ("en_b", &we_b),
            ("data", &q),
            ("data_b", &q_b),
            ("bl", &bl_out),
            ("br", &br_out),
            ("vdd", &vdd),
            ("vss", &vss),
        ]);
        wrdrv.set_name("write_driver");
        ctx.add_instance(wrdrv);

        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.connect_all([
            ("clk", &sense_en),
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

        let mut dff = ctx.instantiate::<StdCell>(&dfrtp.id())?;
        dff.connect_all([
            ("VPWR", vdd),
            ("VGND", vss),
            ("VNB", vss),
            ("VPB", vdd),
            ("CLK", clk),
            ("RESET_B", reset_b),
            ("D", din),
            ("Q", q),
            ("Q_N", q_b),
        ]);
        dff.set_name("dff");
        ctx.add_instance(dff);

        Ok(())
    }
}
