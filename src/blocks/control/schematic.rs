use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use super::{ControlLogicReplicaV2, EdgeDetector, InvChain, SrLatch};

impl ControlLogicReplicaV2 {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        // PORTS
        let [clk, ce, we, reset, decrepend] =
            ctx.ports(["clk", "ce", "we", "reset", "decrepend"], Direction::Input);
        let [saen, pc_b, wlen, wrdrven, decrepstart] = ctx.ports(
            ["saen", "pc_b", "wlen", "wrdrven", "decrepstart"],
            Direction::Output,
        );
        let [rbl, vdd, vss] = ctx.ports(["rbl", "vdd", "vss"], Direction::InOut);

        // SIGNALS
        let [clk_buf, clkp0, clkp, clkp_b, clkpd, clkpd_b, clkpdd, clkp_grst_b] = ctx.signals([
            "clk_buf",
            "clkp0",
            "clkp",
            "clkp_b",
            "clkpd",
            "clkpd_b",
            "clkpdd",
            "clkp_grst_b",
        ]);
        let [wlen_grst_b, wlen_rst_decoderd, wlen_b] =
            ctx.signals(["wlen_grst_b", "wlen_rst_decoderd", "wlen_b"]);
        let [saen_set_b, saen_b] = ctx.signals(["saen_set_b", "saen_b"]);
        let [wrdrven_set_b, wrdrven_grst_b, wrdrven_b] =
            ctx.signals(["wrdrven_set_b", "wrdrven_grst_b", "wrdrven_b"]);
        let [we_b, pc, pc_set_b, rbl_b] = ctx.signals(["we_b", "pc", "pc_set_b", "rbl_b"]);

        // STANDARD CELLS
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let inv = lib.try_cell_named("sky130_fd_sc_hs__inv_2")?;
        let and2 = lib.try_cell_named("sky130_fd_sc_hs__and2_2")?;
        let nand2 = lib.try_cell_named("sky130_fd_sc_hs__nand2_4")?;
        let nor2 = lib.try_cell_named("sky130_fd_sc_hs__nor2_4")?;
        let mux2 = lib.try_cell_named("sky130_fd_sc_hs__mux2_4")?;
        let buf = lib.try_cell_named("sky130_fd_sc_hs__buf_16")?;
        let biginv = lib.try_cell_named("sky130_fd_sc_hs__inv_16")?;

        // CLK LOGIC
        ctx.instantiate::<StdCell>(&and2.id())?
            .with_connections([
                ("A", clk),
                ("B", ce),
                ("X", clk_buf),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("clk_gate")
            .add_to(ctx);
        ctx.instantiate::<EdgeDetector>(&NoParams)?
            .with_connections([
                ("din", clk_buf),
                ("dout", clkp0),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("clk_pulse")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&buf.id())?
            .with_connections([
                ("A", clkp0),
                ("X", clkp),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("clk_pulse_buf")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&biginv.id())?
            .with_connections([
                ("A", clkp),
                ("Y", clkp_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("clk_pulse_inv")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&3)?
            .with_connections([("din", clkp_b), ("dout", clkpd), ("vdd", vdd), ("vss", vss)])
            .named("clkp_delay")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", clkpd),
                ("Y", clkpd_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("clkpd_inv")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&3)?
            .with_connections([
                ("din", clkpd_b),
                ("dout", clkpdd),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("clkpd_delay")
            .add_to(ctx);

        // REPLICA LOGIC
        //
        // Turn on wordlines at start of cycle.
        // Turn them off when replica bitline drops low enough to flip an inverter.
        ctx.instantiate::<StdCell>(&mux2.id())?
            .with_connections([
                ("A0", rbl_b),
                ("A1", clkpdd),
                ("S", we),
                ("X", decrepstart),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("mux_wlen_rst")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&6)?
            .with_connections([
                ("din", decrepend),
                ("dout", wlen_rst_decoderd),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("decoder_replica_delay")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", we),
                ("Y", we_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("inv_we")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", rbl),
                ("Y", rbl_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("inv_rbl")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&nor2.id())?
            .with_connections([
                ("A", decrepstart),
                ("B", reset),
                ("Y", wlen_grst_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("wlen_grst")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&nor2.id())?
            .with_connections([
                ("A", wlen_rst_decoderd),
                ("B", reset),
                ("Y", pc_set_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("pc_set")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&nor2.id())?
            .with_connections([
                ("A", decrepend),
                ("B", reset),
                ("Y", wrdrven_grst_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("wrdrven_grst")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&nor2.id())?
            .with_connections([
                ("A", clkp),
                ("B", reset),
                ("Y", clkp_grst_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("clkp_grst")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&nand2.id())?
            .with_connections([
                ("A", we_b),
                ("B", decrepend),
                ("Y", saen_set_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("and_sense_en")
            .add_to(ctx);

        // CONTROL LATCHES
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("sb", clkpd_b),
                ("rb", wlen_grst_b),
                ("q", wlen),
                ("qb", wlen_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wl_ctl")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("sb", saen_set_b),
                ("rb", clkp_grst_b),
                ("q", saen),
                ("qb", saen_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("saen_ctl")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("sb", pc_set_b),
                ("rb", clkp_b),
                ("q", pc),
                ("qb", pc_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("pc_ctl")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&nand2.id())?
            .with_connections([
                ("A", clkp),
                ("B", we),
                ("Y", wrdrven_set_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("wrdrven_set")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("sb", wrdrven_set_b),
                ("rb", wrdrven_grst_b),
                ("q", wrdrven),
                ("qb", wrdrven_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wrdrven_ctl")
            .add_to(ctx);

        Ok(())
    }
}

impl SrLatch {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let [s, r] = ctx.ports(["sb", "rb"], Direction::Input);
        let [q, qb] = ctx.ports(["q", "qb"], Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);

        let [q0, q0b] = ctx.signals(["q0", "q0b"]);

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_default_lib()?;
        let nand2 = lib.try_cell_named("sky130_fd_sc_hd__nand2_8")?;
        let inv = lib.try_cell_named("sky130_fd_sc_hd__inv_2")?;

        let mut nand_set = ctx.instantiate::<StdCell>(&nand2.id())?;
        let mut nand_reset = nand_set.clone();

        nand_set.connect_all([
            ("A", s),
            ("B", q0b),
            ("Y", q0),
            ("VPWR", vdd),
            ("VPB", vdd),
            ("VGND", vss),
            ("VNB", vss),
        ]);
        nand_set.set_name("nand_set");
        ctx.add_instance(nand_set);

        nand_reset.connect_all([
            ("A", r),
            ("B", q0),
            ("Y", q0b),
            ("VPWR", vdd),
            ("VPB", vdd),
            ("VGND", vss),
            ("VNB", vss),
        ]);
        nand_reset.set_name("nand_reset");
        ctx.add_instance(nand_reset);

        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", q0),
                ("Y", qb),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("qb_inv")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", q0b),
                ("Y", q),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("q_inv")
            .add_to(ctx);

        Ok(())
    }
}

impl InvChain {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let din = ctx.port("din", Direction::Input);
        let dout = ctx.port("dout", Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let x = ctx.bus("x", self.n - 1);

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_default_lib()?;
        let inv = lib.try_cell_named("sky130_fd_sc_hd__inv_2")?;
        let inv_end = lib.try_cell_named("sky130_fd_sc_hd__inv_4")?;

        for i in 0..self.n {
            ctx.instantiate::<StdCell>(&if i == self.n - 1 {
                inv_end.id()
            } else {
                inv.id()
            })?
            .with_connections([
                ("A", if i == 0 { din } else { x.index(i - 1) }),
                ("Y", if i == self.n - 1 { dout } else { x.index(i) }),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named(format!("inv{i}"))
            .add_to(ctx);
        }
        Ok(())
    }
}

impl EdgeDetector {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let din = ctx.port("din", Direction::Input);
        let dout = ctx.port("dout", Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let delayed = ctx.signal("delayed");

        ctx.instantiate::<InvChain>(&self.invs)?
            .with_connections([("din", din), ("dout", delayed), ("vdd", vdd), ("vss", vss)])
            .named("delay_chain")
            .add_to(ctx);

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_default_lib()?;
        let and2 = lib.try_cell_named("sky130_fd_sc_hd__and2_4")?;

        ctx.instantiate::<StdCell>(&and2.id())?
            .with_connections([
                ("A", din),
                ("B", delayed),
                ("X", dout),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("and")
            .add_to(ctx);
        Ok(())
    }
}
