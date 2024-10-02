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
        let and2_med = ctx.instantiate::<StdCell>(&and2_med.id())?;
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
                ("nand_wlendb_web", &nand2),
                ("and_wlen", &and2_med),
                ("wlen_q_delay", &ctx.instantiate::<InvChain>(&3)?),
                ("rwl_buf", &buf),
            ])?,
            outline,
        ));

        let mut row = create_row(&[
            ("saen_ctl", &sr_latch),
            ("pc_ctl", &sr_latch),
            ("wrdrven_set", &nand2),
            ("wrdrven_ctl", &sr_latch),
        ])?;
        row.set_orientation(Named::ReflectVert);
        rows.push(LayerBbox::new(row, outline));

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
                router.block(layer, shape.brect());
            }
        }

        // Pins
        let (num_input_pins, clk_idx, we_idx) = (6usize, 0, 1);
        let num_output_pins = 6usize;
        let mut input_rects = Vec::new();
        let mut output_rects = Vec::new();
        let top_offset = 2;

        let htracks = router.track_info(m1).tracks().clone();
        let htrack_start = htracks.track_with_loc(TrackLocator::EndsBefore, group.brect().top());
        let vtracks = router.track_info(m2).tracks().clone();

        // Input pins
        let vtrack = vtracks
            .index(vtracks.track_with_loc(TrackLocator::EndsBefore, group.brect().left() - 3_200));
        for i in 0..num_input_pins {
            let htrack = htracks.index(htrack_start - 2 * (i as i64) - top_offset);
            input_rects.push(Rect::from_spans(vtrack, htrack));
            ctx.draw_rect(m1, input_rects[i]);
        }

        router.block(
            m2,
            Rect::from_spans(
                vtrack.expand(false, 2000).expand(true, 140),
                group.brect().vspan(),
            ),
        );

        // Output pins
        let vtrack = vtracks
            .index(8 + vtracks.track_with_loc(TrackLocator::StartsAfter, group.brect().right()));
        for i in 0..num_output_pins {
            let htrack = htracks.index(htrack_start - 2 * (i as i64) - top_offset);
            output_rects.push(Rect::from_spans(vtrack, htrack));
            ctx.draw_rect(m1, output_rects[i]);
        }

        router.block(
            m2,
            Rect::from_spans(
                vtrack.expand(true, 2000).expand(false, 140),
                group.brect().vspan(),
            ),
        );

        let clk_pin = input_rects[0];
        router.occupy(m1, clk_pin, "clk")?;
        let ce_pin = input_rects[1];
        router.occupy(m1, ce_pin, "ce")?;
        let we_pin = input_rects[2];
        router.occupy(m1, we_pin, "we")?;
        let resetb_pin = input_rects[3];
        router.occupy(m1, resetb_pin, "reset_b")?;
        let decrepend_pin = input_rects[4];
        router.occupy(m1, decrepend_pin, "decrepend")?;
        let rbl_pin = input_rects[5];
        router.occupy(m1, rbl_pin, "rbl")?;

        let pc_b_pin = output_rects[0];
        router.occupy(m1, pc_b_pin, "pc_b")?;
        let rwl_pin = output_rects[1];
        router.occupy(m1, rwl_pin, "rwl")?;
        let wlen_pin = output_rects[2];
        router.occupy(m1, wlen_pin, "wlen")?;
        let write_driver_en_pin = output_rects[3];
        router.occupy(m1, write_driver_en_pin, "wrdrven")?;
        let saen_pin = output_rects[4];
        router.occupy(m1, saen_pin, "saen")?;
        let decrepstart_pin = output_rects[5];
        router.occupy(m1, decrepstart_pin, "decrepstart")?;

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

        // reset
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
        ctx.draw(jog)?;

        // TODO clk_pulse_inv.y -> clkp_delay.din

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

        // we -> mux_wlen_rst.s
        let we_in_mux = group.port_map().port("mux_wlen_rst_s")?.largest_rect(m0)?;
        let mut via = via01.clone();
        via.align_centers_gridded(we_in_mux.bbox(), grid);
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
        let clkp_grstb_out = group.port_map().port("clkp_grst_a")?.largest_rect(m0)?;
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

        router.route_with_net(ctx, m1, clk_pin, m1, clk_in, "clk")?;
        router.route_with_net(ctx, m1, ce_pin, m1, ce_in, "ce")?;
        router.route_with_net(ctx, m1, resetb_pin, m1, resetb_in, "reset_b")?;
        router.route_with_net(ctx, m1, clkp_b_out, m1, clkp_b_in, "clkp_b")?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in, "we")?;
        router.route_with_net(ctx, m1, rbl_b_out, m1, rbl_b_in, "rbl_b")?;
        router.route_with_net(ctx, m1, wlen_grstb_out, m1, wlen_grstb_in, "wlen_grst_b")?;
        router.route_with_net(ctx, m1, clkp_out, m1, clkp_in, "clkp")?;
        router.route_with_net(ctx, m1, clkp_grstb_out, m1, clkp_grstb_in, "clkp_grst_b")?;
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
        ctx.add_port(CellPort::with_shape("pc_b", m1, pc_b_pin))?;
        ctx.add_port(CellPort::with_shape("rbl", m1, rbl_pin))?;
        ctx.add_port(CellPort::with_shape(
            "write_driver_en",
            m1,
            write_driver_en_pin,
        ))?;
        ctx.add_port(CellPort::with_shape("saen", m1, saen_pin))?;

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

        via.align_centers_gridded(b0.largest_rect(m0)?, grid);
        via.align_left(b0.largest_rect(m0)?);
        let b0_via = via.clone();

        via.align_centers_gridded(y0.largest_rect(m0)?, grid);
        via.align_bottom(y0.largest_rect(m0)?);
        let y0_via = via.clone();

        via.align_left(b1.largest_rect(m0)?);
        via.align_bottom(b1.largest_rect(m0)?);
        let b1_via = via.clone();

        via.align_centers_gridded(y1.largest_rect(m0)?, grid);
        via.align_top(y1.largest_rect(m0)?);
        let y1_via = via.clone();

        via.align_centers_gridded(aq0.largest_rect(m0)?, grid);
        via.align_bottom(aq0.largest_rect(m0)?);
        let aq0_via = via.clone();

        via.align_centers_gridded(aq0b.largest_rect(m0)?, grid);
        via.align_top(aq0b.largest_rect(m0)?);
        via.translate(Point::new(0, -40));
        let aq0b_via = via.clone();

        ctx.draw(
            ElbowJog::builder()
                .src(y0_via.layer_bbox(m1).into_rect().edge(Side::Right))
                .dst(b1_via.brect().center())
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
                .dst(b0_via.brect().center())
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

        ctx.draw(b0_via)?;
        ctx.draw(y0_via)?;
        ctx.draw(b1_via)?;
        ctx.draw(y1_via)?;
        ctx.draw(aq0_via)?;
        ctx.draw(aq0b_via)?;

        ctx.add_port(a0.clone().with_id("sb"))?;
        ctx.add_port(yq.clone().with_id("q"))?;
        ctx.add_port(a1.clone().with_id("rb"))?;
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
                row.push_num(
                    LayerBbox::new(inv.clone(), outline),
                    self.n - (num_groups - 1) * group_size,
                );
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
        b_via.align_centers_gridded(b.bbox(), grid);
        b_via.align_bottom(b.bbox());
        row.add(b_via.clone());

        let dout = row.port_map().port("dout")?.largest_rect(m0)?;
        let mut dout_via = via.with_orientation(Named::R90);
        dout_via.align_centers_gridded(dout.bbox(), grid);
        dout_via.align_centers_vertically_gridded(b_via.bbox(), grid);
        dout_via.translate(Point::new(0, -380));
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
        row.add_group(
            ElbowJog::builder()
                .src(dout_via.layer_bbox(m1).into_rect().edge(Side::Right))
                .dst(b_via.brect().center())
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
