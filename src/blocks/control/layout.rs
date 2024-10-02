use arcstr::ArcStr;
use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::index::IndexOwned;
use substrate::layout::cell::{CellPort, Instance, Port, PortConflictStrategy, PortId};
use substrate::layout::elements::mos::LayoutMos;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::group::Group;
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;
use substrate::layout::placement::align::{AlignMode, AlignRect};
use substrate::layout::placement::array::{ArrayTiler, ArrayTilerBuilder};
use substrate::layout::placement::tile::LayerBbox;
use substrate::layout::routing::auto::grid::ExpandToGridStrategy;
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::manual::jog::{ElbowJog, SJog};
use substrate::layout::routing::tracks::TrackLocator;
use substrate::layout::Draw;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};
use substrate::pdk::stdcell::StdCell;

use super::{ControlLogicReplicaV2, EdgeDetector, InvChain, SrLatch};
use subgeom::transform::Translate;
use subgeom::{Corner, Dir, Point, Rect, Side, Span};

impl ControlLogicReplicaV2 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let stdcells = ctx.inner().std_cell_db();
        let db = ctx.mos_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let inv = lib.try_cell_named("sky130_fd_sc_hs__inv_2")?;
        let inv = ctx.instantiate::<StdCell>(&inv.id())?;
        let tap = lib.try_cell_named("sky130_fd_sc_hs__tap_2")?;
        let tap = ctx.instantiate::<StdCell>(&tap.id())?;
        let tap = LayerBbox::new(tap, outline);
        let and2 = lib.try_cell_named("sky130_fd_sc_hs__and2_2")?;
        let and2 = ctx.instantiate::<StdCell>(&and2.id())?;
        let and2_med = lib.try_cell_named("sky130_fd_sc_hs__and2_4")?;
        let mut and2_med = ctx.instantiate::<StdCell>(&and2_med.id())?;
        and2_med.reflect_horiz_anchored();
        let nand2 = lib.try_cell_named("sky130_fd_sc_hs__nand2_4")?;
        let nand2 = ctx.instantiate::<StdCell>(&nand2.id())?;
        let nor2 = lib.try_cell_named("sky130_fd_sc_hs__nor2_4")?;
        let nor2 = ctx.instantiate::<StdCell>(&nor2.id())?;
        let mux2 = lib.try_cell_named("sky130_fd_sc_hs__mux2_4")?;
        let mut mux2 = ctx.instantiate::<StdCell>(&mux2.id())?;
        mux2.reflect_horiz_anchored();
        let buf = lib.try_cell_named("sky130_fd_sc_hs__buf_16")?;
        let mut buf = ctx.instantiate::<StdCell>(&buf.id())?;
        buf.reflect_horiz_anchored();
        let biginv = lib.try_cell_named("sky130_fd_sc_hs__inv_16")?;
        let biginv = ctx.instantiate::<StdCell>(&biginv.id())?;
        let edge_detector = ctx.instantiate::<EdgeDetector>(&NoParams)?;
        let sr_latch = ctx.instantiate::<SrLatch>(&NoParams)?;

        let mut rows = ArrayTiler::builder();
        rows.mode(AlignMode::Left).alt_mode(AlignMode::Beneath);

        let create_row = |insts: &[(&str, &Instance)]| -> substrate::error::Result<Group> {
            let mut row = new_row();
            row.push(tap.clone());
            for (_, inst) in insts {
                row.push(LayerBbox::new((*inst).clone(), outline));
                row.push(tap.clone());
            }
            let mut row = row.build();

            let names: Vec<String> = insts.iter().map(|(name, _)| name.to_string()).collect();
            row.expose_ports(
                |port: CellPort, i| {
                    let name = if i % 2 == 1 {
                        &names[i / 2]
                    } else {
                        return None;
                    };
                    if let "vpwr" | "vgnd" | "vdd" | "vss" = port.name().as_str() {
                        return None;
                    }
                    let port_name = format!("{}_{}", name, port.name());
                    Some(port.named(port_name))
                },
                PortConflictStrategy::Error,
            )?;
            row.expose_ports(
                |port: CellPort, _| match port.name().as_str() {
                    "vpwr" => Some(port.named("vdd")),
                    "vgnd" => Some(port.named("vss")),
                    "vdd" | "vss" => Some(port),
                    _ => None,
                },
                PortConflictStrategy::Merge,
            )?;

            row.generate()
        };

        rows.push(LayerBbox::new(
            create_row(&[
                ("reset_inv", &biginv),
                ("clk_gate", &and2),
                ("clk_pulse", &edge_detector),
                ("clk_pulse_buf", &buf),
                ("clk_pulse_inv", &biginv),
            ])?,
            outline,
        ));

        let mut row = create_row(&[
            ("inv_rbl", &inv),
            ("clkp_delay", &ctx.instantiate::<InvChain>(&3)?),
            ("clkpd_inv", &inv),
            ("clkpd_delay", &ctx.instantiate::<InvChain>(&7)?),
            ("mux_wlen_rst", &mux2),
            ("decoder_replica_delay", &ctx.instantiate::<InvChain>(&6)?),
            ("wl_ctl", &sr_latch),
        ])?;
        row.set_orientation(Named::ReflectVert);
        rows.push(LayerBbox::new(row, outline));

        rows.push(LayerBbox::new(
            create_row(&[
                ("inv_we", &inv),
                ("wlen_grst", &nor2),
                ("pc_set", &nor2),
                ("wrdrven_grst", &nor2),
                ("clkp_grst", &nor2),
                ("nand_sense_en", &nand2),
                ("wlen_q_delay", &ctx.instantiate::<InvChain>(&3)?),
                ("nand_wlendb_web", &nand2),
                ("and_wlen", &and2_med),
                ("rwl_buf", &buf),
            ])?,
            outline,
        ));

        let mut row = create_row(&[
            ("saen_ctl", &sr_latch),
            ("wrdrven_set", &nand2),
            ("wrdrven_ctl", &sr_latch),
        ])?;
        row.set_orientation(Named::ReflectVert);
        rows.push(LayerBbox::new(row, outline));

        rows.push(LayerBbox::new(
            create_row(&[
                ("decoder_replica", &ctx.instantiate::<InvChain>(&16)?),
                ("pc_ctl", &sr_latch),
            ])?,
            outline,
        ));

        let mut rows = rows.build();
        rows.expose_ports(
            |port: CellPort, _| {
                if let "vdd" | "vss" = port.name().as_str() {
                    None
                } else {
                    Some(port)
                }
            },
            PortConflictStrategy::Error,
        )?;
        rows.expose_ports(
            |port: CellPort, _| {
                if let "vdd" | "vss" = port.name().as_str() {
                    Some(port)
                } else {
                    None
                }
            },
            PortConflictStrategy::Merge,
        )?;
        let mut group = rows.generate()?;

        self.route(ctx, &group)?;

        ctx.add_ports(
            group
                .ports()
                .filter(|port| matches!(port.name().as_str(), "vdd" | "vss")),
        )?;

        ctx.draw(group)?;
        Ok(())
    }

    fn route(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
        group: &Group,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let via01 = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m0, m1)
                .geometry(
                    Rect::from_point(Point::zero()),
                    Rect::from_point(Point::zero()),
                )
                .bot_extension(Dir::Vert)
                .top_extension(Dir::Vert)
                .build(),
        )?;
        let via12 = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m1, m2)
                .geometry(
                    Rect::from_point(Point::zero()),
                    Rect::from_point(Point::zero()),
                )
                .bot_extension(Dir::Vert)
                .top_extension(Dir::Vert)
                .build(),
        )?;

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: group.brect().expand(8 * 680),
            layers: vec![
                LayerConfig {
                    line: 320,
                    space: 140,
                    dir: Dir::Horiz,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 140,
                    dir: Dir::Vert,
                    layer: m2,
                },
            ],
        });

        let grid = ctx.pdk().layout_grid();

        let vss = group.port_map().port("vss")?.first_rect(m1, Side::Top)?;
        let mut vss_rect = Rect::from_spans(Span::until(200), vss.brect().vspan());
        vss_rect.align_right(vss);
        vss_rect = router.expand_to_grid(vss_rect, ExpandToGridStrategy::Minimum);
        router.occupy(m1, vss_rect, "vss")?;
        ctx.draw_rect(m1, vss_rect);

        for layer in [m1, m2] {
            for shape in group.shapes_on(layer) {
                let rect = shape.brect().expand(40);
                router.block(layer, rect);
            }
        }

        // Pins
        let num_left_pins = 4;
        let num_bot_pins = 6;
        let mut left_pins = Vec::new();
        let mut bot_pins = Vec::new();
        let top_offset = 2;
        let left_offset = 50;

        let htracks = router.track_info(m1).tracks().clone();
        let htrack_start = htracks.track_with_loc(TrackLocator::EndsBefore, group.brect().top());
        let vtracks = router.track_info(m2).tracks().clone();
        let vtrack_start = vtracks.track_with_loc(TrackLocator::EndsBefore, group.brect().left());

        // left pins
        let vtrack = vtracks
            .index(vtracks.track_with_loc(TrackLocator::EndsBefore, group.brect().left() - 3_200));
        for i in 0..num_left_pins {
            let htrack = htracks.index(htrack_start - 2 * (i as i64) - top_offset);
            left_pins.push(Rect::from_spans(vtrack, htrack));
            ctx.draw_rect(m1, left_pins[i]);
        }

        router.block(
            m2,
            Rect::from_spans(
                vtrack.expand(false, 2000).expand(true, 140),
                group.brect().vspan(),
            ),
        );

        // bot pins
        let htrack = htracks
            .index(htracks.track_with_loc(TrackLocator::EndsBefore, group.brect().bottom()) - 8);
        for i in 0..num_bot_pins {
            let vtrack = vtracks.index(vtrack_start + 2 * (i as i64) + left_offset);
            bot_pins.push(Rect::from_spans(vtrack, htrack));
            ctx.draw_rect(m2, bot_pins[i]);
        }

        router.block(
            m1,
            Rect::from_spans(
                group.brect().hspan(),
                htrack.expand(true, 140).expand(false, 2000),
            ),
        );

        let clk_pin = left_pins[0];
        router.occupy(m1, clk_pin, "clk")?;
        let ce_pin = left_pins[1];
        router.occupy(m1, ce_pin, "ce")?;
        let we_pin = left_pins[2];
        router.occupy(m1, we_pin, "we")?;
        let resetb_pin = left_pins[3];
        router.occupy(m1, resetb_pin, "reset_b")?;

        let rbl_pin = bot_pins[0];
        router.occupy(m2, rbl_pin, "rbl")?;
        let rwl_pin = bot_pins[1];
        router.occupy(m2, rwl_pin, "rwl")?;
        let pc_b_pin = bot_pins[2];
        router.occupy(m2, pc_b_pin, "pc_b")?;
        let wlen_pin = bot_pins[3];
        router.occupy(m2, wlen_pin, "wlen")?;
        let wrdrven_pin = bot_pins[4];
        router.occupy(m2, wrdrven_pin, "wrdrven")?;
        let saen_pin = bot_pins[5];
        router.occupy(m2, saen_pin, "saen")?;

        // reset_b -> reset_inv.y
        let resetb_in = group.port_map().port("reset_inv_a")?.largest_rect(m1)?;
        let resetb_in =
            router.expand_to_grid(resetb_in, ExpandToGridStrategy::Corner(Corner::LowerLeft));
        ctx.draw_rect(m1, resetb_in);
        router.occupy(m1, resetb_in, "reset_b")?;

        // clk -> clk_gate.a
        let clk_in = group.port_map().port("clk_gate_a")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(clk_in.bbox(), grid);
        let clk_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::LowerLeft),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, clk_in);
        router.occupy(m1, clk_in, "clk")?;

        // ce -> clk_gate.b
        let ce_in = group.port_map().port("clk_gate_b")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(ce_in.bbox(), grid);
        let ce_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, ce_in);
        router.occupy(m1, ce_in, "ce")?;

        // rbl -> inv_rbl.a
        let pin = group.port_map().port("inv_rbl_a")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(pin.bbox(), grid);
        let rbl_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperLeft),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, rbl_in);
        router.occupy(m1, rbl_in, "rbl")?;

        // reset out
        let reset_out = group.port_map().port("reset_inv_y")?.largest_rect(m1)?;
        let reset_out =
            router.expand_to_grid(reset_out, ExpandToGridStrategy::Corner(Corner::UpperRight));
        ctx.draw_rect(m1, reset_out);
        router.occupy(m1, reset_out, "reset")?;

        // clk_gate.x -> clk_pulse.din
        let clk_gate_out = group.port_map().port("clk_gate_x")?.largest_rect(m0)?;
        let clk_pulse_in = group.port_map().port("clk_pulse_din")?.largest_rect(m0)?;
        ctx.draw_rect(
            m0,
            Rect::from_spans(
                Span::new(clk_gate_out.right(), clk_pulse_in.left()),
                Span::from_center_span_gridded(clk_pulse_in.vspan().center(), 180, 10),
            ),
        );

        // wlen_q_delay.dout -> nand_wlendb_web.a
        let wlendb_out = group
            .port_map()
            .port("wlen_q_delay_dout")?
            .largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(wlendb_out.bbox(), grid);
        let wlendb_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::LowerLeft),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wlendb_out);
        router.occupy(m1, wlendb_out, "wlend_b")?;

        let wlendb_in = group
            .port_map()
            .port("nand_wlendb_web_a")?
            .largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(wlendb_in.bbox(), grid);
        let wlendb_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::LowerLeft),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wlendb_in);
        router.occupy(m1, wlendb_in, "wlend_b")?;

        // nand_wlendb_web.y -> and_wlen.b
        let wlend_out = group
            .port_map()
            .port("nand_wlendb_web_y")?
            .largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(wlend_out.bbox(), grid);
        let wlend_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::LowerLeft),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wlend_out);
        router.occupy(m1, wlend_out, "wlend")?;

        let wlend_in = group.port_map().port("and_wlen_b")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(wlend_in.bbox(), grid);
        via.align_right(wlend_in.bbox());
        let wlend_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wlend_in);
        router.occupy(m1, wlend_in, "wlend")?;

        // clk_pulse.dout -> clk_pulse_buf.a
        let src = group.port_map().port("clk_pulse_dout")?.largest_rect(m0)?;
        let dst = group.port_map().port("clk_pulse_buf_a")?.largest_rect(m0)?;
        ctx.draw_rect(
            m0,
            Rect::from_spans(
                Span::new(src.right(), dst.left()),
                Span::from_center_span_gridded(dst.vspan().center(), 180, 10),
            ),
        );

        // clk_pulse_inv.y -> clkp_delay.din
        let clkp_b_out = group.port_map().port("clk_pulse_inv_y")?.largest_rect(m1)?;
        let clkp_b_out =
            router.expand_to_grid(clkp_b_out, ExpandToGridStrategy::Corner(Corner::UpperRight));
        ctx.draw_rect(m1, clkp_b_out);
        router.occupy(m1, clkp_b_out, "clkp_b")?;

        let clkp_b_in = group.port_map().port("clkp_delay_din")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(clkp_b_in.bbox(), grid);
        let clkp_b_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, clkp_b_in);
        router.occupy(m1, clkp_b_in, "clkp_b")?;

        // clkp_b -> pc_ctl.rb
        let pin = group.port_map().port("pc_ctl_rb")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(pin.bbox(), grid);
        let clkp_b_in_1 = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Right),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, clkp_b_in_1);
        router.occupy(m1, clkp_b_in_1, "clkp_b")?;

        // clkp_delay.dout -> wrdrven_set.a
        let clkpd_out = group.port_map().port("clkp_delay_dout")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(clkpd_out.bbox(), grid);
        let clkpd_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, clkpd_out);
        router.occupy(m1, clkpd_out, "clkpd")?;

        let clkpd_in = group.port_map().port("wrdrven_set_a")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(clkpd_in.bbox(), grid);
        let clkpd_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, clkpd_in);
        router.occupy(m1, clkpd_in, "clkpd")?;

        let port = group.port_map().port("wl_ctl_q")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(port.bbox(), grid);
        let wl_ctl_q_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wl_ctl_q_out);
        router.occupy(m1, wl_ctl_q_out, "wlen_q")?;

        let mut snap_pins = |net: &str, pins: &[&str]| -> substrate::error::Result<Vec<Rect>> {
            let mut out_pins = Vec::with_capacity(pins.len());
            for &pin in pins {
                let port = group.port_map().port(pin)?.largest_rect(m0)?;
                let mut via = via01.clone();
                via.align_centers_gridded(port.bbox(), grid);
                let port = router.expand_to_grid(
                    via.layer_bbox(m1).into_rect(),
                    ExpandToGridStrategy::Corner(Corner::UpperRight),
                );
                ctx.draw(via)?;
                ctx.draw_rect(m1, port);
                router.occupy(m1, port, net)?;
                out_pins.push(port);
            }
            Ok(out_pins)
        };

        // we_b
        let we_b_ins = snap_pins("we_b", &["nand_sense_en_a", "nand_wlendb_web_b"])?;

        // wlen_q
        let mut wlen_qs = snap_pins("wlen_q", &["and_wlen_a", "wlen_q_delay_din", "rwl_buf_a"])?;
        wlen_qs.push(wl_ctl_q_out);

        // saen_set_bs
        let saen_set_bs = snap_pins("saen_set_b", &["nand_sense_en_y", "saen_ctl_sb"])?;

        // clkpd_b
        let clkpd_bs = snap_pins("clkpd_b", &["clkpd_inv_y", "wl_ctl_sb"])?;

        // wrdrven_grst_b
        let wrdrven_grst_bs = snap_pins("wrdrven_grst_b", &["wrdrven_grst_y", "wrdrven_ctl_rb"])?;

        // decrepstart
        let decrepstarts = snap_pins(
            "decrepstart",
            &["mux_wlen_rst_x", "decoder_replica_din", "wlen_grst_a"],
        )?;

        // decrepend
        let decrepends = snap_pins(
            "decrepend",
            &[
                "decoder_replica_dout",
                "decoder_replica_delay_din",
                "wrdrven_grst_a",
                "nand_sense_en_b",
            ],
        )?;

        // we -> wrdrven_set.b
        let we_in_1 = group.port_map().port("wrdrven_set_b")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(we_in_1.bbox(), grid);
        let we_in_1 = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, we_in_1);
        router.occupy(m1, we_in_1, "we")?;

        // wrdrven_set.y -> wrdrven_ctl.sb
        let wrdrven_set_out = group.port_map().port("wrdrven_set_y")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(wrdrven_set_out.bbox(), grid);
        let wrdrven_set_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wrdrven_set_out);
        router.occupy(m1, wrdrven_set_out, "wrdrven_set_b")?;

        let wrdrven_set_in = group.port_map().port("wrdrven_ctl_sb")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(wrdrven_set_in.bbox(), grid);
        let wrdrven_set_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wrdrven_set_in);
        router.occupy(m1, wrdrven_set_in, "wrdrven_set_b")?;

        // clk_pulse_buf.x -> clk_pulse_inv.a
        let src = group.port_map().port("clk_pulse_buf_x")?.largest_rect(m1)?;
        let dst = group.port_map().port("clk_pulse_inv_a")?.largest_rect(m1)?;
        let jog = SJog::builder()
            .src(src)
            .dst(dst)
            .width(230)
            .grid(10)
            .layer(m1)
            .dir(Dir::Horiz)
            .build()
            .unwrap();
        router.block(m1, jog.r1());
        router.block(m1, jog.r2());
        router.block(m1, jog.r3());
        ctx.draw(jog)?;

        // clkp_delay.dout -> clkpd_inv.a
        let src = group.port_map().port("clkp_delay_dout")?.largest_rect(m0)?;
        let dst = group.port_map().port("clkpd_inv_a")?.largest_rect(m0)?;
        ctx.draw_rect(
            m0,
            Rect::from_spans(
                Span::new(src.right(), dst.left()),
                Span::from_center_span_gridded(dst.vspan().center(), 180, 10),
            ),
        );

        // clkpd_inv.y -> clkpd_delay.din
        let src = group.port_map().port("clkpd_inv_y")?.largest_rect(m0)?;
        let dst = group.port_map().port("clkpd_delay_din")?.largest_rect(m0)?;
        ctx.draw_rect(
            m0,
            Rect::from_spans(
                Span::new(src.right(), dst.left()),
                Span::from_center_span_gridded(dst.vspan().center(), 180, 10),
            ),
        );

        // clkpd_delay.dout -> mux_wlen_rst.a1
        let src = group
            .port_map()
            .port("clkpd_delay_dout")?
            .largest_rect(m0)?;
        let dst = group.port_map().port("mux_wlen_rst_a1")?.largest_rect(m0)?;
        let mut clkpdd_out_via = via01.clone();
        clkpdd_out_via.align_centers_gridded(src, grid);
        let mut clkpdd_in_via = via01.clone();
        clkpdd_in_via.align_centers_gridded(dst, grid);
        let rect = clkpdd_out_via
            .layer_bbox(m1)
            .union(clkpdd_in_via.layer_bbox(m1))
            .into_rect();
        ctx.draw(clkpdd_out_via)?;
        ctx.draw(clkpdd_in_via)?;
        ctx.draw_rect(m1, rect);
        router.occupy(m1, rect, "clkpdd")?;

        // rwl_buf.x
        let pin = group.port_map().port("rwl_buf_x")?.largest_rect(m1)?;
        let rwl_out = router.expand_to_grid(pin, ExpandToGridStrategy::Corner(Corner::UpperRight));
        ctx.draw_rect(m1, rwl_out);
        router.occupy(m1, rwl_out, "rwl")?;

        // and_wlen.x
        let pin = group.port_map().port("and_wlen_x")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(pin.bbox(), grid);
        let wlen_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wlen_out);
        router.occupy(m1, wlen_out, "wlen")?;

        // saen_ctl.q
        let pin = group.port_map().port("saen_ctl_q")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(pin.bbox(), grid);
        let saen_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, saen_out);
        router.occupy(m1, saen_out, "saen")?;

        // wrdrven_ctl.q
        let pin = group.port_map().port("wrdrven_ctl_q")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(pin.bbox(), grid);
        let wrdrven_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, wrdrven_out);
        router.occupy(m1, wrdrven_out, "wrdrven")?;

        // pc_ctl.qb
        let pin = group.port_map().port("pc_ctl_qb")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(pin.bbox(), grid);
        let pc_b_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, pc_b_out);
        router.occupy(m1, pc_b_out, "pc_b")?;

        // inv_we.a
        let we_in_inv = group.port_map().port("inv_we_a")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(we_in_inv.bbox(), grid);
        let we_in_inv = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::LowerLeft),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, we_in_inv);
        router.occupy(m1, we_in_inv, "we")?;

        // inv_we.y
        let port = group.port_map().port("inv_we_y")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(port.bbox(), grid);
        let we_b_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, we_b_out);
        router.occupy(m1, we_b_out, "we_b")?;

        // we -> mux_wlen_rst.s
        let we_in = group.port_map().port("mux_wlen_rst_s")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(we_in.bbox(), grid);
        let we_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, we_in);
        router.occupy(m1, we_in, "we")?;

        // inv_rbl.y -> mux_wlen_rst.a0
        let rbl_b_out = group.port_map().port("inv_rbl_y")?.largest_rect(m0)?;
        let mut rbl_b_out_via = via01.clone();
        rbl_b_out_via.align_centers_gridded(rbl_b_out.bbox(), grid);
        let rbl_b_out = router.expand_to_grid(
            rbl_b_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(rbl_b_out_via)?;
        ctx.draw_rect(m1, rbl_b_out);
        router.occupy(m1, rbl_b_out, "rbl_b")?;

        let rbl_b_in = group.port_map().port("mux_wlen_rst_a0")?.largest_rect(m0)?;
        let mut rbl_b_in_via = via01.clone();
        rbl_b_in_via.align_centers_gridded(rbl_b_in.bbox(), grid);
        let rbl_b_in = router.expand_to_grid(
            rbl_b_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(rbl_b_in_via)?;
        ctx.draw_rect(m1, rbl_b_in);
        router.occupy(m1, rbl_b_in, "rbl_b")?;

        // TODO mux_wlen_rst.x -> decrepstart
        // TODO decrepend -> decoder_replica_delay.din

        // wlen_grst_b
        let wlen_grstb_out = group.port_map().port("wlen_grst_y")?.largest_rect(m0)?;
        let mut wlen_grstb_out_via = via01.clone();
        wlen_grstb_out_via.align_centers_gridded(wlen_grstb_out.bbox(), grid);
        let wlen_grstb_out = router.expand_to_grid(
            wlen_grstb_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(wlen_grstb_out_via)?;
        ctx.draw_rect(m1, wlen_grstb_out);
        router.occupy(m1, wlen_grstb_out, "wlen_grst_b")?;
        let wlen_grstb_in = group.port_map().port("wl_ctl_rb")?.largest_rect(m0)?;
        let mut wlen_grstb_in_via = via01.clone();
        wlen_grstb_in_via.align_centers_gridded(wlen_grstb_in.bbox(), grid);
        let wlen_grstb_in = router.expand_to_grid(
            wlen_grstb_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(wlen_grstb_in_via)?;
        ctx.draw_rect(m1, wlen_grstb_in);
        router.occupy(m1, wlen_grstb_in, "wlen_grst_b")?;

        // wlen_rst_decoderd
        let wlen_rst_decoderd_out = group
            .port_map()
            .port("decoder_replica_delay_dout")?
            .largest_rect(m0)?;
        let mut wlen_rst_decoderd_out_via = via01.clone();
        wlen_rst_decoderd_out_via.align_centers_gridded(wlen_rst_decoderd_out.bbox(), grid);
        let wlen_rst_decoderd_out = router.expand_to_grid(
            wlen_rst_decoderd_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(wlen_rst_decoderd_out_via)?;
        ctx.draw_rect(m1, wlen_rst_decoderd_out);
        router.occupy(m1, wlen_rst_decoderd_out, "wlen_rst_decoderd")?;

        let wlen_rst_decoderd_in = group.port_map().port("pc_set_a")?.largest_rect(m0)?;
        let mut wlen_rst_decoderd_in_via = via01.clone();
        wlen_rst_decoderd_in_via.align_centers_gridded(wlen_rst_decoderd_in.bbox(), grid);
        let wlen_rst_decoderd_in = router.expand_to_grid(
            wlen_rst_decoderd_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(wlen_rst_decoderd_in_via)?;
        ctx.draw_rect(m1, wlen_rst_decoderd_in);
        router.occupy(m1, wlen_rst_decoderd_in, "wlen_rst_decoderd")?;

        // reset
        let mut resets = Vec::new();
        for pin in ["wlen_grst_b", "pc_set_b", "wrdrven_grst_b", "clkp_grst_b"] {
            let in_pin = group.port_map().port(pin)?.largest_rect(m0)?;
            let mut via = via01.clone();
            via.align_centers_gridded(in_pin.bbox(), grid);
            let in_pin = router.expand_to_grid(
                via.layer_bbox(m1).into_rect(),
                ExpandToGridStrategy::Corner(Corner::UpperRight),
            );
            ctx.draw(via)?;
            ctx.draw_rect(m1, in_pin);
            router.occupy(m1, in_pin, "reset")?;
            resets.push(in_pin);
        }

        // clkp
        let clkp_out = group.port_map().port("clk_pulse_buf_x")?.largest_rect(m1)?;
        let clkp_out =
            router.expand_to_grid(clkp_out, ExpandToGridStrategy::Corner(Corner::UpperRight));
        ctx.draw_rect(m1, clkp_out);
        router.occupy(m1, clkp_out, "clkp")?;

        let clkp_in = group.port_map().port("clkp_grst_a")?.largest_rect(m0)?;
        let mut clkp_in_via = via01.clone();
        clkp_in_via.align_centers_gridded(clkp_in.bbox(), grid);
        let clkp_in = router.expand_to_grid(
            clkp_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(clkp_in_via)?;
        ctx.draw_rect(m1, clkp_in);
        router.occupy(m1, clkp_in, "clkp")?;

        // clkp_grst_b
        let clkp_grstb_out = group.port_map().port("clkp_grst_y")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(clkp_grstb_out.bbox(), grid);
        let clkp_grstb_out = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, clkp_grstb_out);
        router.occupy(m1, clkp_grstb_out, "clkp_grst_b")?;

        let clkp_grstb_in = group.port_map().port("saen_ctl_rb")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(clkp_grstb_in.bbox(), grid);
        let clkp_grstb_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, clkp_grstb_in);
        router.occupy(m1, clkp_grstb_in, "clkp_grst_b")?;

        // pc_set_b
        let pc_setb_out = group.port_map().port("pc_set_y")?.largest_rect(m0)?;
        let mut pc_setb_out_via = via01.clone();
        pc_setb_out_via.align_centers_gridded(pc_setb_out.bbox(), grid);
        let pc_setb_out = router.expand_to_grid(
            pc_setb_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(pc_setb_out_via)?;
        ctx.draw_rect(m1, pc_setb_out);
        router.occupy(m1, pc_setb_out, "pc_set_b")?;

        let pin = group.port_map().port("pc_ctl_sb")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(pin.bbox(), grid);
        let pc_setb_in = router.expand_to_grid(
            via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(via)?;
        ctx.draw_rect(m1, pc_setb_in);
        router.occupy(m1, pc_setb_in, "pc_set_b")?;

        router.route_with_net(ctx, m1, clk_pin, m1, clk_in, "clk")?;
        router.route_with_net(ctx, m1, ce_pin, m1, ce_in, "ce")?;
        router.route_with_net(ctx, m1, pc_b_out, m2, pc_b_pin, "pc_b")?;
        router.route_with_net(ctx, m1, resetb_pin, m1, resetb_in, "reset_b")?;
        router.route_with_net(ctx, m1, clkp_b_out, m1, clkp_b_in, "clkp_b")?;
        router.route_with_net(ctx, m1, clkp_b_out, m1, clkp_b_in_1, "clkp_b")?;
        router.route_with_net(ctx, m1, clkpd_out, m1, clkpd_in, "clkpd")?;
        router.route_with_net(ctx, m1, pc_setb_out, m1, pc_setb_in, "pc_set_b")?;
        for reset_in in resets {
            router.route_with_net(ctx, m1, reset_out, m1, reset_in, "reset")?;
        }
        for we_b_in in we_b_ins {
            router.route_with_net(ctx, m1, we_b_out, m1, we_b_in, "we_b")?;
        }
        let mut route_pins = |pins: &[(&str, &[Rect])]| -> substrate::error::Result<()> {
            for (net, rects) in pins {
                for dst in &rects[1..] {
                    router.route_with_net(ctx, m1, rects[0], m1, *dst, net)?;
                }
            }
            Ok(())
        };
        route_pins(&[
            ("wlen_q", &wlen_qs),
            ("decrepstart", &decrepstarts),
            ("decrepend", &decrepends),
            ("wrdrven_grst_b", &wrdrven_grst_bs),
            ("clkpd_b", &clkpd_bs),
            ("saen_set_b", &saen_set_bs),
        ])?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in, "we")?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in_1, "we")?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in_inv, "we")?;
        router.route_with_net(ctx, m1, rbl_b_out, m1, rbl_b_in, "rbl_b")?;
        router.route_with_net(ctx, m1, rwl_out, m2, rwl_pin, "rwl")?;
        router.route_with_net(ctx, m1, wlen_grstb_out, m1, wlen_grstb_in, "wlen_grst_b")?;
        router.route_with_net(ctx, m1, clkp_out, m1, clkp_in, "clkp")?;
        router.route_with_net(ctx, m1, clkp_grstb_out, m1, clkp_grstb_in, "clkp_grst_b")?;
        router.route_with_net(ctx, m1, wlendb_out, m1, wlendb_in, "wlend_b")?;
        router.route_with_net(ctx, m1, wlend_out, m1, wlend_in, "wlend")?;
        router.route_with_net(ctx, m1, wlen_out, m2, wlen_pin, "wlen")?;
        router.route_with_net(ctx, m1, saen_out, m2, saen_pin, "saen")?;
        router.route_with_net(ctx, m2, wrdrven_pin, m1, wrdrven_out, "wrdrven")?;
        router.route_with_net(ctx, m2, rbl_pin, m1, rbl_in, "rbl")?;
        router.route_with_net(
            ctx,
            m1,
            wrdrven_set_out,
            m1,
            wrdrven_set_in,
            "wrdrven_set_b",
        )?;
        router.route_with_net(
            ctx,
            m1,
            wlen_rst_decoderd_out,
            m1,
            wlen_rst_decoderd_in,
            "wlen_rst_decoderd",
        )?;

        ctx.draw(router)?;

        ctx.add_port(CellPort::with_shape("clk", m1, clk_pin))?;
        ctx.add_port(CellPort::with_shape("ce", m1, ce_pin))?;
        ctx.add_port(CellPort::with_shape("we", m1, we_pin))?;
        ctx.add_port(CellPort::with_shape("reset_b", m1, resetb_pin))?;

        ctx.add_port(CellPort::with_shape("pc_b", m2, pc_b_pin))?;
        ctx.add_port(CellPort::with_shape("rbl", m2, rbl_pin))?;
        ctx.add_port(CellPort::with_shape("wrdrven", m2, wrdrven_pin))?;
        ctx.add_port(CellPort::with_shape("saen", m2, saen_pin))?;
        ctx.add_port(CellPort::with_shape("wlen", m2, wlen_pin))?;
        ctx.add_port(CellPort::with_shape("rwl", m2, rwl_pin))?;

        Ok(())
    }
}

fn new_row<'a>() -> ArrayTilerBuilder<'a> {
    let mut row = ArrayTiler::builder();
    row.mode(AlignMode::ToTheRight).alt_mode(AlignMode::Top);
    row
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvChainsParams {
    chains: Vec<(String, usize)>,
    wrap_cutoff: usize,
    flipped: bool,
}

pub struct InvChains {
    params: InvChainsParams,
}

impl Component for InvChains {
    type Params = InvChainsParams;
    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> ArcStr {
        arcstr::format!("inv_chains")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let tap = lib.try_cell_named("sky130_fd_sc_hs__tap_2")?;
        let tap = ctx.instantiate::<StdCell>(&tap.id())?;

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let grid = ctx.pdk().layout_grid();

        let mut invs = ArrayTiler::builder();
        invs.mode(AlignMode::Left).alt_mode(AlignMode::Beneath);

        let mut flipped = self.params.flipped;
        for (name, n) in &self.params.chains {
            let mut inv_current = ArrayTiler::builder();
            inv_current
                .mode(AlignMode::Left)
                .alt_mode(AlignMode::Beneath);

            let n = *n;
            let one_row = n <= self.params.wrap_cutoff;
            assert!(
                one_row || n % 2 == 0,
                "only even length wrapping inverter chains are currently supported"
            );

            let row_len = if one_row { n } else { n / 2 };
            let mut inv_chain = ctx.instantiate::<InvChain>(&row_len)?;
            if flipped {
                inv_chain.set_orientation(Named::ReflectVert);
            }
            let mut row = new_row();
            let row_tap = tap.with_orientation(if flipped {
                Named::ReflectVert
            } else {
                Named::Default
            });
            row.push(LayerBbox::new(row_tap.clone(), outline));
            row.push(LayerBbox::new(inv_chain.clone(), outline));
            row.push(LayerBbox::new(row_tap, outline));
            let mut row = row.build();
            row.expose_ports(
                |port: CellPort, _| match port.name().as_str() {
                    "vpwr" => Some(port.named("vdd")),
                    "vgnd" => Some(port.named("vss")),
                    _ => Some(port),
                },
                PortConflictStrategy::Merge,
            )?;
            inv_current.push(LayerBbox::new(row.generate()?, outline));
            flipped = !flipped;

            if one_row {
                let mut inv_current = inv_current.build();
                inv_current.expose_ports(
                    |port: CellPort, _| {
                        if port.name().as_str() == "din" {
                            Some(port.with_id(format!("{}_din", name)))
                        } else if port.name().as_str() == "dout" {
                            Some(port.with_id(format!("{}_dout", name)))
                        } else {
                            Some(port)
                        }
                    },
                    PortConflictStrategy::Merge,
                )?;
                invs.push(LayerBbox::new(inv_current.generate()?, outline));
                continue;
            }

            inv_chain.set_orientation(if flipped {
                Named::R180
            } else {
                Named::ReflectHoriz
            });
            let mut row = new_row();
            let row_tap = tap.with_orientation(if flipped {
                Named::ReflectVert
            } else {
                Named::Default
            });
            row.push(LayerBbox::new(row_tap.clone(), outline));
            row.push(LayerBbox::new(inv_chain.clone(), outline));
            row.push(LayerBbox::new(row_tap, outline));
            let mut row = row.build();
            row.expose_ports(
                |port: CellPort, _| match port.name().as_str() {
                    "vpwr" => Some(port.named("vdd")),
                    "vgnd" => Some(port.named("vss")),
                    _ => Some(port),
                },
                PortConflictStrategy::Merge,
            )?;
            inv_current.push(LayerBbox::new(row.generate()?, outline));
            flipped = !flipped;

            // Route two halves of inverter chain to one another.
            let mut via01 = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m0, m1)
                    .geometry(
                        Rect::from_point(Point::zero()),
                        Rect::from_point(Point::zero()),
                    )
                    .bot_extension(Dir::Vert)
                    .top_extension(Dir::Vert)
                    .build(),
            )?;
            let mut via12 = ctx.instantiate::<Via>(
                &ViaParams::builder()
                    .layers(m1, m2)
                    .geometry(
                        Rect::from_point(Point::zero()),
                        Rect::from_point(Point::zero()),
                    )
                    .bot_extension(Dir::Vert)
                    .top_extension(Dir::Vert)
                    .build(),
            )?;

            let mut inv_current = inv_current.build();
            inv_current.expose_ports(
                |port: CellPort, i| match (i, port.name().as_str()) {
                    (0, "din") => Some(port.with_id(format!("{}_din", name))),
                    (1, "dout") => Some(port.with_id(format!("{}_dout", name))),
                    (_, "din" | "dout") => Some(port.named("tmp").with_index(i)),
                    _ => Some(port),
                },
                PortConflictStrategy::Merge,
            )?;
            let mut inv_group = inv_current.generate()?;
            let out_port = inv_group
                .port_map()
                .port(PortId::new("tmp", 0))?
                .largest_rect(m0)?;
            let in_port = inv_group
                .port_map()
                .port(PortId::new("tmp", 1))?
                .largest_rect(m0)?;

            via01.align_centers_gridded(out_port.bbox(), grid);
            via12.align_centers_gridded(out_port.bbox(), grid);
            let out_via = via12.clone();
            inv_group.add(via01.clone());
            inv_group.add(out_via.clone());

            via01.align_centers_gridded(in_port.bbox(), grid);
            via12.align_centers_gridded(in_port.bbox(), grid);
            let in_via = via12.with_orientation(Named::R90);
            inv_group.add(via01.with_orientation(Named::R90));
            inv_group.add(in_via.clone());

            inv_group.add_group(
                ElbowJog::builder()
                    .src(out_via.layer_bbox(m1).into_rect().edge(Side::Bot))
                    .dst(in_via.brect().center())
                    .layer(m2)
                    .build()
                    .unwrap()
                    .draw()?,
            );
            invs.push(LayerBbox::new(inv_group, outline));
        }
        let mut invs = invs.build();
        invs.expose_ports(
            |port: CellPort, _| {
                if port.name().as_str() != "tmp" {
                    Some(port)
                } else {
                    None
                }
            },
            PortConflictStrategy::Merge,
        )?;

        ctx.add_ports(invs.ports().cloned())?;
        ctx.draw(invs)?;

        Ok(())
    }
}

impl SrLatch {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let nand2 = lib.try_cell_named("sky130_fd_sc_hs__nand2_8")?;
        let nand2 = ctx.instantiate::<StdCell>(&nand2.id())?;
        let nand2_hflip = nand2.with_orientation(Named::ReflectHoriz);
        let inv = lib.try_cell_named("sky130_fd_sc_hs__inv_2")?;
        let inv = ctx.instantiate::<StdCell>(&inv.id())?;

        let layers = ctx.inner().layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let grid = ctx.pdk().layout_grid();

        let nand2 = LayerBbox::new(nand2, outline);
        let nand2_hflip = LayerBbox::new(nand2_hflip, outline);
        let inv = LayerBbox::new(inv, outline);
        let mut via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m0, m1)
                .geometry(
                    Rect::from_point(Point::zero()),
                    Rect::from_point(Point::zero()),
                )
                .bot_extension(Dir::Horiz)
                .build(),
        )?;

        let mut row = new_row();

        row.push(nand2);
        row.push(nand2_hflip);
        row.push(inv.clone());
        row.push(inv);

        let mut row = row.build();
        row.expose_ports(
            |port: CellPort, i| Some(port.with_index(i)),
            PortConflictStrategy::Error,
        )?;
        let a0 = row.port_map().port(PortId::new("a", 0))?;
        let b0 = row.port_map().port(PortId::new("b", 0))?;
        let y0 = row.port_map().port(PortId::new("y", 0))?;
        let a1 = row.port_map().port(PortId::new("a", 1))?;
        let b1 = row.port_map().port(PortId::new("b", 1))?;
        let y1 = row.port_map().port(PortId::new("y", 1))?;
        let aq0b = row.port_map().port(PortId::new("a", 2))?;
        let yq = row.port_map().port(PortId::new("y", 2))?;
        let aq0 = row.port_map().port(PortId::new("a", 3))?;
        let yqb = row.port_map().port(PortId::new("y", 3))?;

        via.align_left(a0.largest_rect(m0)?);
        via.align_top(a0.largest_rect(m0)?);
        let a0_via = via.clone();

        via.align_centers_gridded(y0.largest_rect(m0)?, grid);
        via.align_bottom(y0.largest_rect(m0)?);
        via.translate(Point::new(0, -360));
        let y0_via = via.clone();

        via.align_left(a1.largest_rect(m0)?);
        via.align_bottom(a1.largest_rect(m0)?);
        let a1_via = via.clone();

        via.align_centers_gridded(y1.largest_rect(m0)?, grid);
        via.align_top(y1.largest_rect(m0)?);
        via.translate(Point::new(0, 100));
        let y1_via = via.clone();

        via.align_centers_gridded(aq0.largest_rect(m0)?, grid);
        via.align_bottom(aq0.largest_rect(m0)?);
        let aq0_via = via.clone();

        via.align_top(aq0b.largest_rect(m0)?);
        via.align_left(aq0b.largest_rect(m0)?);
        via.translate(Point::new(0, -40));
        let aq0b_via = via.clone();

        ctx.draw(
            ElbowJog::builder()
                .src(y0_via.layer_bbox(m1).into_rect().edge(Side::Right))
                .dst(a1_via.brect().center())
                .layer(m1)
                .build()
                .unwrap(),
        )?;
        ctx.draw(
            ElbowJog::builder()
                .src(y0_via.layer_bbox(m1).into_rect().edge(Side::Right))
                .dst(aq0_via.brect().center())
                .layer(m1)
                .build()
                .unwrap(),
        )?;

        ctx.draw(
            ElbowJog::builder()
                .src(y1_via.layer_bbox(m1).into_rect().edge(Side::Left))
                .dst(a0_via.brect().center())
                .layer(m1)
                .build()
                .unwrap(),
        )?;
        ctx.draw(
            ElbowJog::builder()
                .src(y1_via.layer_bbox(m1).into_rect().edge(Side::Left))
                .dst(aq0b_via.brect().center())
                .layer(m1)
                .build()
                .unwrap(),
        )?;

        ctx.draw(a0_via)?;
        ctx.draw(y0_via)?;
        ctx.draw(a1_via)?;
        ctx.draw(y1_via)?;
        ctx.draw(aq0_via)?;
        ctx.draw(aq0b_via)?;

        ctx.add_port(b0.clone().with_id("sb"))?;
        ctx.add_port(yq.clone().with_id("q"))?;
        ctx.add_port(b1.clone().with_id("rb"))?;
        ctx.add_port(yqb.clone().with_id("qb"))?;

        for i in 0..4 {
            let vpwr = row.port_map().port(PortId::new("vpwr", i))?;
            let vgnd = row.port_map().port(PortId::new("vgnd", i))?;
            ctx.merge_port(vpwr.clone().with_id("vdd"));
            ctx.merge_port(vgnd.clone().with_id("vss"));
        }

        ctx.draw(row)?;

        Ok(())
    }
}

impl InvChain {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let inv = lib.try_cell_named("sky130_fd_sc_hs__inv_2")?;
        let inv = ctx.instantiate::<StdCell>(&inv.id())?;
        let inv_end = lib.try_cell_named("sky130_fd_sc_hs__inv_4")?;
        let inv_end = ctx.instantiate::<StdCell>(&inv_end.id())?;
        let tap = lib.try_cell_named("sky130_fd_sc_hs__tap_2")?;
        let tap = ctx.instantiate::<StdCell>(&tap.id())?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let outline = layers.get(Selector::Name("outline"))?;

        let mut row = new_row();
        let group_size = 8;
        let num_groups = self.n.div_ceil(group_size);
        for i in 0..num_groups {
            if i == num_groups - 1 {
                let rem = self.n - (num_groups - 1) * group_size;
                row.push_num(
                    LayerBbox::new(inv.clone(), outline),
                    rem.checked_sub(1).unwrap(),
                );
                row.push(LayerBbox::new(inv_end.clone(), outline));
            } else {
                row.push_num(LayerBbox::new(inv.clone(), outline), group_size);
                row.push(LayerBbox::new(tap.clone(), outline));
            }
        }
        let mut row = row.build();
        row.expose_ports(
            |port: CellPort, i| {
                if i % (group_size + 1) < group_size {
                    Some(port.with_index(i / (group_size + 1) * group_size + i % (group_size + 1)))
                } else {
                    None
                }
            },
            PortConflictStrategy::Error,
        )?;

        for i in 0..self.n - 1 {
            let y = row.port_map().port(PortId::new("y", i))?.largest_rect(m0)?;
            let a = row
                .port_map()
                .port(PortId::new("a", i + 1))?
                .largest_rect(m0)?;
            ctx.draw_rect(
                m0,
                Rect::from_spans(
                    a.hspan().union(y.hspan()),
                    Span::from_center_span_gridded(a.center().y, 170, 10),
                ),
            );
        }
        ctx.add_port(
            row.port_map()
                .port(PortId::new("a", 0))?
                .clone()
                .with_id("din"),
        )?;
        ctx.add_port(
            row.port_map()
                .port(PortId::new("y", self.n - 1))?
                .clone()
                .with_id("dout"),
        )?;
        for i in 0..self.n {
            ctx.merge_port(
                row.port_map()
                    .port(PortId::new("vpwr", i))?
                    .clone()
                    .with_id("vdd"),
            );
            ctx.merge_port(
                row.port_map()
                    .port(PortId::new("vgnd", i))?
                    .clone()
                    .with_id("vss"),
            );
        }

        ctx.draw(row)?;
        Ok(())
    }
}

impl EdgeDetector {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hs")?;
        let and = lib.try_cell_named("sky130_fd_sc_hs__and2_4")?;
        let mut and = ctx.instantiate::<StdCell>(&and.id())?;
        and.reflect_horiz_anchored();

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let grid = ctx.pdk().layout_grid();

        let inv_chain = ctx.instantiate::<InvChain>(&self.invs)?;

        let mut row = new_row();
        row.push(LayerBbox::new(inv_chain, outline));
        row.push(LayerBbox::new(and, outline));
        let mut row = row.build();
        row.expose_ports(
            |port: CellPort, i| Some(port.with_index(i)),
            PortConflictStrategy::Error,
        )?;
        let mut row = row.generate()?;

        // Routing.
        let via = ctx.instantiate::<Via>(
            &ViaParams::builder()
                .layers(m0, m1)
                .geometry(
                    Rect::from_point(Point::zero()),
                    Rect::from_point(Point::zero()),
                )
                .bot_extension(Dir::Vert)
                .top_extension(Dir::Vert)
                .build(),
        )?;
        let din = row.port_map().port("din")?.largest_rect(m0)?;
        let mut din_via = via.clone();
        din_via.align_centers_gridded(din.bbox(), grid);
        row.add(din_via.clone());

        let a = row.port_map().port(PortId::new("a", 1))?.largest_rect(m0)?;
        let mut a_via = via.with_orientation(Named::R90);
        a_via.align_centers_gridded(a.bbox(), grid);
        a_via.align_top(a.bbox());
        row.add(a_via.clone());

        let b = row.port_map().port(PortId::new("b", 1))?.largest_rect(m0)?;
        let mut b_via = via.with_orientation(Named::R90);
        b_via.align_left(b.bbox());
        b_via.align_bottom(b.bbox());
        row.add(b_via.clone());

        let dout = row.port_map().port("dout")?.largest_rect(m0)?;
        let mut dout_via = via.with_orientation(Named::R90);
        dout_via.align_centers_gridded(dout.bbox(), grid);
        dout_via.align_centers_vertically_gridded(b_via.bbox(), grid);
        dout_via.translate(Point::new(0, -340));
        row.add(dout_via.clone());

        row.add_group(
            ElbowJog::builder()
                .src(a_via.layer_bbox(m1).into_rect().edge(Side::Left))
                .dst(din_via.brect().center())
                .layer(m1)
                .build()
                .unwrap()
                .draw()?,
        );
        let src = dout_via.layer_bbox(m1).into_rect().edge(Side::Right);
        let len = src.span().length();
        row.add_group(
            ElbowJog::builder()
                .src(src)
                .dst(
                    b_via
                        .brect()
                        .center()
                        .translated(Point::new(-(290 - len) / 2, 0)),
                )
                .layer(m1)
                .build()
                .unwrap()
                .draw()?,
        );

        ctx.add_port(row.port_map().port("din")?.clone())?;
        ctx.add_port(
            row.port_map()
                .port(PortId::new("x", 1))?
                .clone()
                .with_id("dout"),
        )?;

        let vdd0 = row.port_map().port(PortId::new("vdd", 0))?;
        let vss0 = row.port_map().port(PortId::new("vss", 0))?;
        let vpwr1 = row.port_map().port(PortId::new("vpwr", 1))?;
        let vgnd1 = row.port_map().port(PortId::new("vgnd", 1))?;
        ctx.merge_port(vdd0.clone());
        ctx.merge_port(vss0.clone());
        ctx.merge_port(vpwr1.clone().with_id("vdd"));
        ctx.merge_port(vgnd1.clone().with_id("vss"));

        ctx.draw(row)?;
        Ok(())
    }
}
