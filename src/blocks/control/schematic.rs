use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::elements::mos::SchematicMos;

use super::{ControlLogicKind, ControlLogicReplicaV2, EdgeDetector, InvChain, SrLatch};

impl ControlLogicReplicaV2 {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        // PORTS
        let [clk, we] = ctx.ports(["clk", "we"], Direction::Input);
        let [pc_b, wl_en0, wl_en, write_driver_en, sense_en] = ctx.ports(
            ["pc_b", "wl_en0", "wl_en", "write_driver_en", "sense_en"],
            Direction::Output,
        );
        let [rbl, dummy_bl, vdd, vss] =
            ctx.ports(["rbl", "dummy_bl", "vdd", "vss"], Direction::InOut);

        // SIGNALS
        let [clk_b, clk_buf, clkp] = ctx.signals(["clk_b", "clk_buf", "clkp"]);
        let [pc_read_set, pc_set, pc_b0, pc, pc_write_set] =
            ctx.signals(["pc_read_set", "pc_set", "pc_b0", "pc", "pc_write_set"]);
        let [wl_en_set, wl_en_rst, wl_en0_b, wl_en_write_rst] =
            ctx.signals(["wl_en_set", "wl_en_rst", "wl_en0_b", "wl_en_write_rst"]);
        let [sense_en_set0, sense_en_set, sense_en_b, sae_set] =
            ctx.signals(["sense_en_set0", "sense_en_set", "sense_en_b", "sae_set"]);

        let sae_int = match self.0 {
            ControlLogicKind::Standard => ctx.signal("sae_int"),
            ControlLogicKind::Test => ctx.port("sae_int", Direction::Output),
        };

        let [wr_drv_set, wr_drv_set_undelayed] =
            ctx.signals(["wr_drv_set", "wr_drv_set_undelayed"]);
        let [rbl_b] = ctx.signals(["rbl_b"]);
        let [write_driver_en0, write_driver_en_b] =
            ctx.signals(["write_driver_en0", "write_driver_en_b"]);
        let [we_b, dummy_bl_b, wbl_pulldown_en] =
            ctx.signals(["we_b", "dummy_bl_b", "wbl_pulldown_en"]);

        // STANDARD CELLS
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_default_lib()?;
        let inv = lib.try_cell_named("sky130_fd_sc_hd__inv_2")?;
        let and2 = lib.try_cell_named("sky130_fd_sc_hd__and2_2")?;
        let mux2 = lib.try_cell_named("sky130_fd_sc_hd__mux2_2")?;
        let bufbuf = lib.try_cell_named("sky130_fd_sc_hd__bufbuf_8")?;

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        // CLK LOGIC
        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", clk),
                ("Y", clk_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("inv_clk")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", clk_b),
                ("Y", clk_buf),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("inv_clk_b")
            .add_to(ctx);
        ctx.instantiate::<EdgeDetector>(&NoParams)?
            .with_connections([("din", clk_buf), ("dout", clkp), ("vdd", vdd), ("vss", vss)])
            .named("clk_pulse")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&8)?
            .with_connections([
                ("din", clkp),
                ("dout", wl_en_set),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("decoder_replica")
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

        // REPLICA LOGIC
        //
        // Turn on wordlines at start of cycle.
        // Turn them off when replica bitline drops low enough to flip an inverter.
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
        ctx.instantiate::<InvChain>(&8)?
            .with_connections([
                ("din", rbl_b),
                ("dout", pc_read_set),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("pc_read_set_buf")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&and2.id())?
            .with_connections([
                ("A", we_b),
                ("B", rbl_b),
                ("X", sense_en_set0),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("and_sense_en")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&2)?
            .with_connections([
                ("din", sense_en_set0),
                ("dout", sense_en_set),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("sense_en_delay")
            .add_to(ctx);

        // CONTROL LATCHES
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("s", wl_en_set),
                ("r", wl_en_rst),
                ("q", wl_en0),
                ("qb", wl_en0_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wl_ctl")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("s", sense_en_set),
                ("r", clkp),
                ("q", sae_int),
                ("qb", sense_en_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("sae_ctl")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("s", pc_set),
                ("r", clkp),
                ("q", pc),
                ("qb", pc_b0),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("pc_ctl")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&NoParams)?
            .with_connections([
                ("s", wr_drv_set),
                ("r", wl_en_write_rst),
                ("q", write_driver_en0),
                ("qb", write_driver_en_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wr_drv_ctl")
            .add_to(ctx);

        ctx.instantiate::<StdCell>(&mux2.id())?
            .with_connections([
                ("A0", rbl_b),
                ("A1", wl_en_write_rst),
                ("S", we),
                ("X", wl_en_rst),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("mux_wl_en_rst")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&mux2.id())?
            .with_connections([
                ("A0", pc_read_set),
                ("A1", pc_write_set),
                ("S", we),
                ("X", pc_set),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("mux_pc_set")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&and2.id())?
            .with_connections([
                ("A", we_b),
                ("B", rbl_b),
                ("X", sae_set),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("sae_set")
            .add_to(ctx);

        ctx.instantiate::<StdCell>(&and2.id())?
            .with_connections([
                ("A", clkp),
                ("B", we),
                ("X", wr_drv_set_undelayed),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("wr_drv_set")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&24)?
            .with_connections([
                ("din", wr_drv_set_undelayed),
                ("dout", wr_drv_set),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wr_drv_set_decoder_delay_replica")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&inv.id())?
            .with_connections([
                ("A", dummy_bl),
                ("Y", dummy_bl_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("inv_dummy_bl")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&4)?
            .with_connections([
                ("din", dummy_bl_b),
                ("dout", wl_en_write_rst),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wl_en_write_rst_buf")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&4)?
            .with_connections([
                ("din", wl_en_write_rst),
                ("dout", pc_write_set),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("pc_write_set_buf")
            .add_to(ctx);

        // BUFFERS
        ctx.instantiate::<StdCell>(&bufbuf.id())?
            .with_connections([
                ("A", wl_en0),
                ("X", wl_en),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("wl_en_buf")
            .add_to(ctx);

        let sae_buf_in = match self.0 {
            ControlLogicKind::Standard => sae_int,
            ControlLogicKind::Test => ctx.port("sae_muxed", Direction::Input),
        };
        ctx.instantiate::<StdCell>(&bufbuf.id())?
            .with_connections([
                ("A", sae_buf_in),
                ("X", sense_en),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("sae_buf")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&bufbuf.id())?
            .with_connections([
                ("A", sae_buf_in),
                ("X", sense_en),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("sae_buf2")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&bufbuf.id())?
            .with_connections([
                ("A", pc_b0),
                ("X", pc_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("pc_b_buf")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&bufbuf.id())?
            .with_connections([
                ("A", pc_b0),
                ("X", pc_b),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("pc_b_buf2")
            .add_to(ctx);
        ctx.instantiate::<StdCell>(&bufbuf.id())?
            .with_connections([
                ("A", write_driver_en0),
                ("X", write_driver_en),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("wr_drv_buf")
            .add_to(ctx);

        ctx.instantiate::<StdCell>(&and2.id())?
            .with_connections([
                ("A", wl_en),
                ("B", write_driver_en),
                ("X", wbl_pulldown_en),
                ("VPWR", vdd),
                ("VPB", vdd),
                ("VGND", vss),
                ("VNB", vss),
            ])
            .named("wbl_pulldown_en")
            .add_to(ctx);
        ctx.instantiate::<SchematicMos>(&MosParams {
            w: 420,
            l: 150,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?
        .with_connections([
            ("d", dummy_bl),
            ("g", wbl_pulldown_en),
            ("s", vss),
            ("b", vss),
        ])
        .named("dummy_bl_pulldown")
        .add_to(ctx);

        Ok(())
    }
}

impl SrLatch {
    pub(crate) fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let [s, r] = ctx.ports(["s", "r"], Direction::Input);
        let [q, qb] = ctx.ports(["q", "qb"], Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_default_lib()?;
        let nor2 = lib.try_cell_named("sky130_fd_sc_hd__nor2_2")?;

        let mut nor_set = ctx.instantiate::<StdCell>(&nor2.id())?;
        let mut nor_reset = nor_set.clone();

        nor_set.connect_all([
            ("A", s),
            ("B", q),
            ("Y", qb),
            ("VPWR", vdd),
            ("VPB", vdd),
            ("VGND", vss),
            ("VNB", vss),
        ]);
        nor_set.set_name("nor_set");
        ctx.add_instance(nor_set);

        nor_reset.connect_all([
            ("A", r),
            ("B", qb),
            ("Y", q),
            ("VPWR", vdd),
            ("VPB", vdd),
            ("VGND", vss),
            ("VNB", vss),
        ]);
        nor_reset.set_name("nor_reset");
        ctx.add_instance(nor_reset);

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

        for i in 0..self.n {
            ctx.instantiate::<StdCell>(&inv.id())?
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
