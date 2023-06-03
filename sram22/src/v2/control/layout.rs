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
use substrate::layout::routing::manual::jog::ElbowJog;
use substrate::layout::routing::tracks::TrackLocator;
use substrate::layout::Draw;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams, MosParams};
use substrate::pdk::stdcell::StdCell;

use subgeom::{Corner, Dir, Point, Rect, Side, Span};

use super::{ControlLogicKind, ControlLogicReplicaV2, EdgeDetector, InvChain, SrLatch};

impl ControlLogicReplicaV2 {
    pub(crate) fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;

        let stdcells = ctx.inner().std_cell_db();
        let db = ctx.mos_db();
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;
        let inv = lib.try_cell_named("sky130_fd_sc_hd__inv_2")?;
        let inv = ctx.instantiate::<StdCell>(&inv.id())?;
        let tap = lib.try_cell_named("sky130_fd_sc_hd__tap_2")?;
        let tap = ctx.instantiate::<StdCell>(&tap.id())?;
        let tap = LayerBbox::new(tap, outline);
        let and = lib.try_cell_named("sky130_fd_sc_hd__and2_2")?;
        let and = ctx.instantiate::<StdCell>(&and.id())?;
        let mux = lib.try_cell_named("sky130_fd_sc_hd__mux2_2")?;
        let mux = ctx.instantiate::<StdCell>(&mux.id())?;
        let bufbuf = lib.try_cell_named("sky130_fd_sc_hd__bufbuf_16")?;
        let bufbuf = ctx.instantiate::<StdCell>(&bufbuf.id())?;
        let edge_detector = ctx.instantiate::<EdgeDetector>(&NoParams)?;
        let sr_latch = ctx.instantiate::<SrLatch>(&NoParams)?;
        let nmos = db.default_nmos().unwrap();
        let mut dummy_bl_pulldown = ctx.instantiate::<LayoutMos>(&LayoutMosParams {
            skip_sd_metal: vec![vec![]],
            deep_nwell: false,
            contact_strategy: GateContactStrategy::SingleSide,
            devices: vec![MosParams {
                w: 420,
                l: 150,
                m: 1,
                nf: 1,
                id: nmos.id(),
            }],
        })?;

        let mut rows = ArrayTiler::builder();
        rows.mode(AlignMode::Left).alt_mode(AlignMode::Beneath);

        let create_row = |insts: &[(&str, &Instance)]| -> substrate::error::Result<Group> {
            let mut row = new_row();
            row.push(tap.clone());
            for (_, inst) in insts {
                row.push(LayerBbox::new((*inst).clone(), outline));
            }
            row.push(tap.clone());
            let mut row = row.build();

            let names: Vec<String> = insts.iter().map(|(name, _)| name.to_string()).collect();
            row.expose_ports(
                |port: CellPort, i| {
                    let name = if i > 0 && i <= names.len() {
                        &names[i - 1]
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
                ("inv_clk", &ctx.instantiate::<InvChain>(&2)?),
                ("clk_pulse", &edge_detector),
                ("decoder_replica", &ctx.instantiate::<InvChain>(&8)?),
                ("inv_rbl", &inv),
                ("and_sense_en", &and),
                ("inv_we", &inv),
            ])?,
            outline,
        ));

        let mut row = create_row(&[
            ("mux_wl_en_rst", &mux),
            ("mux_pc_set", &mux),
            ("wl_ctl", &sr_latch),
            ("sae_ctl", &sr_latch),
            ("pc_ctl", &sr_latch),
            ("wr_drv_ctl", &sr_latch),
            ("sae_set", &and),
        ])?;
        row.set_orientation(Named::ReflectVert);
        rows.push(LayerBbox::new(row, outline));

        rows.push(LayerBbox::new(
            create_row(&[
                ("wr_drv_set", &and),
                ("wl_en_buf", &bufbuf),
                ("sae_buf", &bufbuf),
                ("wbl_pulldown_en", &and),
            ])?,
            outline,
        ));

        let mut row = create_row(&[
            ("sae_buf2", &bufbuf),
            ("pc_b_buf", &bufbuf),
            ("wl_en_write_rst_buf", &ctx.instantiate::<InvChain>(&5)?),
        ])?;
        row.set_orientation(Named::ReflectVert);
        rows.push(LayerBbox::new(row, outline));

        rows.push(LayerBbox::new(
            create_row(&[("pc_b_buf2", &bufbuf), ("wr_drv_buf", &bufbuf)])?,
            outline,
        ));

        let inv_chain_data: Vec<(String, usize)> = [
            ("pc_read_set_buf", 8),
            ("sense_en_delay", 2),
            ("wr_drv_set_decoder_delay_replica", 24),
            ("pc_write_set_buf", 4),
        ]
        .into_iter()
        .map(|(name, n)| (name.to_string(), n))
        .collect();

        let inv_chains = ctx.instantiate::<InvChains>(&InvChainsParams {
            chains: inv_chain_data,
            wrap_cutoff: 24,
            flipped: true,
        })?;

        rows.push(LayerBbox::new(inv_chains, outline));

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

        dummy_bl_pulldown.align_top(group.brect());
        dummy_bl_pulldown.align(AlignMode::Right, group.brect(), -400);

        let mut pulldown_group = Group::new();
        pulldown_group.add(dummy_bl_pulldown);
        pulldown_group.expose_ports(
            |port: CellPort, _| {
                let name = format!("dummy_bl_pulldown_{}", port.name());
                Some(port.named(name))
            },
            PortConflictStrategy::Error,
        )?;
        group.add_ports(pulldown_group.ports())?;
        group.add_group(pulldown_group);

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
        let (num_input_pins, clk_idx, we_idx) = match self.0 {
            ControlLogicKind::Standard => (2usize, 0, 1),
            ControlLogicKind::Test => (4usize, 2, 3),
        };
        let num_output_pins = 7usize;
        let mut input_rects = Vec::new();
        let mut output_rects = Vec::new();
        let top_offset = 2;

        let htracks = router.track_info(m1).tracks().clone();
        let htrack_start = htracks.track_with_loc(TrackLocator::EndsBefore, group.brect().top());
        let vtracks = router.track_info(m2).tracks().clone();

        // Input pins
        let vtrack =
            vtracks.index(vtracks.track_with_loc(TrackLocator::EndsBefore, group.brect().left()));
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

        let sae_muxed = input_rects[0];
        router.occupy(m1, sae_muxed, "sae_muxed")?;
        let sae_int = input_rects[1];
        router.occupy(m1, sae_int, "sae_int")?;
        let clk_pin = input_rects[clk_idx];
        router.occupy(m1, clk_pin, "clk")?;
        let we_pin = input_rects[we_idx];
        router.occupy(m1, we_pin, "we")?;

        let pc_b_pin = output_rects[0];
        router.occupy(m1, pc_b_pin, "pc_b")?;
        let wl_en0_pin = output_rects[1];
        router.occupy(m1, wl_en0_pin, "wl_en0")?;
        let wl_en_pin = output_rects[2];
        router.occupy(m1, wl_en_pin, "wl_en")?;
        let write_driver_en_pin = output_rects[3];
        router.occupy(m1, write_driver_en_pin, "write_driver_en")?;
        let sense_en_pin = output_rects[4];
        router.occupy(m1, sense_en_pin, "sense_en")?;
        let rbl_pin = output_rects[5];
        router.occupy(m1, rbl_pin, "rbl")?;
        let dummy_bl_pin = output_rects[6];
        router.occupy(m1, dummy_bl_pin, "dummy_bl")?;

        // inv_clk -> clk_pulse
        let clk_buf_out = group.port_map().port("inv_clk_dout")?.largest_rect(m0)?;
        let clk_buf_in = group.port_map().port("clk_pulse_din")?.largest_rect(m0)?;
        ctx.draw_rect(
            m0,
            Rect::from_spans(
                clk_buf_in.hspan().union(clk_buf_out.hspan()),
                clk_buf_in.vspan(),
            ),
        );

        // clk_pulse -> decoder_replica
        let clkp_out = group.port_map().port("clk_pulse_dout")?.largest_rect(m0)?;
        let clkp_in = group
            .port_map()
            .port("decoder_replica_din")?
            .largest_rect(m0)?;
        ctx.draw_rect(
            m0,
            Rect::from_spans(clkp_in.hspan().union(clkp_out.hspan()), clkp_in.vspan()),
        );

        // clk_pulse -> sae_ctl/pc_ctl/wr_drv_set
        let mut clkp_out_via = via01.clone();
        clkp_out_via.align_centers_gridded(clkp_out.brect(), grid);
        let clkp_out = router.expand_to_grid(
            clkp_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Left),
        );
        ctx.draw(clkp_out_via)?;
        ctx.draw_rect(m1, clkp_out);
        router.occupy(m1, clkp_out, "clkp")?;

        let clkp_in_1 = group.port_map().port("sae_ctl_r")?.largest_rect(m0)?;
        let mut clkp_in_1_via = via01.clone();
        clkp_in_1_via.align_centers_gridded(clkp_in_1.bbox(), grid);
        clkp_in_1_via.align_left(clkp_in_1.bbox());
        let clkp_in_1 = router.expand_to_grid(
            clkp_in_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(clkp_in_1_via)?;
        ctx.draw_rect(m1, clkp_in_1);
        router.occupy(m1, clkp_in_1, "clkp")?;

        let clkp_in_2 = group.port_map().port("pc_ctl_r")?.largest_rect(m0)?;
        let mut clkp_in_2_via = via01.clone();
        clkp_in_2_via.align_centers_gridded(clkp_in_2.bbox(), grid);
        clkp_in_2_via.align_left(clkp_in_2.bbox());
        let clkp_in_2 = router.expand_to_grid(
            clkp_in_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(clkp_in_2_via)?;
        ctx.draw_rect(m1, clkp_in_2);
        router.occupy(m1, clkp_in_2, "clkp")?;

        let clkp_in_3 = group.port_map().port("wr_drv_set_a")?.largest_rect(m0)?;
        let mut clkp_in_3_via = via01.with_orientation(Named::R90);
        clkp_in_3_via.align_centers_gridded(clkp_in_3.bbox(), grid);
        clkp_in_3_via.align_top(clkp_in_3.bbox());
        let clkp_in_3 = router.expand_to_grid(
            clkp_in_3_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Top),
        );
        ctx.draw(clkp_in_3_via)?;
        ctx.draw_rect(m1, clkp_in_3);
        router.occupy(m1, clkp_in_3, "clkp")?;

        // decoder_replica -> wl_ctl
        let wl_en_set_out = group
            .port_map()
            .port("decoder_replica_dout")?
            .largest_rect(m0)?;
        let mut wl_en_set_out_via = via01.clone();
        wl_en_set_out_via.align_centers_gridded(wl_en_set_out.bbox(), grid);
        let wl_en_set_out = router.expand_to_grid(
            wl_en_set_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Left),
        );
        ctx.draw(wl_en_set_out_via)?;
        ctx.draw_rect(m1, wl_en_set_out);
        router.occupy(m1, wl_en_set_out, "wl_en_set")?;

        let wl_en_set_in = group.port_map().port("wl_ctl_s")?.largest_rect(m0)?;
        let mut wl_en_set_in_via = via01.with_orientation(Named::R90);
        wl_en_set_in_via.align_centers_gridded(wl_en_set_in.bbox(), grid);
        wl_en_set_in_via.align_left(wl_en_set_in.bbox());
        let wl_en_set_in = router.expand_to_grid(
            wl_en_set_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en_set_in_via)?;
        ctx.draw_rect(m1, wl_en_set_in);
        router.occupy(m1, wl_en_set_in, "wl_en_set")?;

        // inv_we -> and_sense_en/sae_set
        let we_b_out = group.port_map().port("inv_we_y")?.largest_rect(m0)?;
        let we_b_in_1 = group.port_map().port("and_sense_en_b")?.largest_rect(m0)?;

        let mut we_b_out_via = via01.clone();
        we_b_out_via.align_centers_gridded(we_b_out.bbox(), grid);
        we_b_out_via.align_centers_vertically_gridded(we_b_in_1.bbox(), grid);
        let we_b_out = router.expand_to_grid(
            we_b_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Top),
        );
        ctx.draw(we_b_out_via)?;
        ctx.draw_rect(m1, we_b_out);
        router.occupy(m1, we_b_out, "we_b")?;

        let mut we_b_in_1_via = via01.with_orientation(Named::R90);
        we_b_in_1_via.align_centers_gridded(we_b_in_1.bbox(), grid);
        let we_b_in_1 = router.expand_to_grid(
            we_b_in_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(we_b_in_1_via)?;
        ctx.draw_rect(m1, we_b_in_1);
        router.occupy(m1, we_b_in_1, "we_b")?;

        let we_b_in_2 = group.port_map().port("sae_set_a")?.largest_rect(m0)?;
        let mut we_b_in_2_via = via01.with_orientation(Named::R90);
        we_b_in_2_via.align_centers_gridded(we_b_in_2.bbox(), grid);
        we_b_in_2_via.align_top(we_b_in_2.bbox());
        let we_b_in_2 = router.expand_to_grid(
            we_b_in_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(we_b_in_2_via)?;
        ctx.draw_rect(m1, we_b_in_2);
        router.occupy(m1, we_b_in_2, "we_b")?;

        // inv_rbl -> and_sense_en
        let rbl_b_out = group.port_map().port("inv_rbl_y")?.largest_rect(m0)?;
        let rbl_b_in = group.port_map().port("and_sense_en_a")?.largest_rect(m0)?;
        ctx.draw_rect(
            m0,
            Rect::from_spans(
                rbl_b_out.hspan().union(rbl_b_in.hspan()),
                Span::with_start_and_length(rbl_b_in.bottom(), 250),
            ),
        );

        // inv_rbl -> pc_read_set_buf/mux_wl_en_rst
        let mut rbl_b_out_via = via01.clone();
        rbl_b_out_via.align_centers_gridded(rbl_b_out.bbox(), grid);
        let rbl_b_out = router.expand_to_grid(
            rbl_b_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(rbl_b_out_via)?;
        ctx.draw_rect(m1, rbl_b_out);
        router.occupy(m1, rbl_b_out, "rbl_b")?;

        let rbl_b_in_2 = group
            .port_map()
            .port("pc_read_set_buf_din")?
            .largest_rect(m0)?;
        let mut rbl_b_in_2_via = via01.with_orientation(Named::R90);
        rbl_b_in_2_via.align_centers_gridded(rbl_b_in_2.bbox(), grid);
        let rbl_b_in_2 = router.expand_to_grid(
            rbl_b_in_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(rbl_b_in_2_via)?;
        ctx.draw_rect(m1, rbl_b_in_2);
        router.occupy(m1, rbl_b_in_2, "rbl_b")?;

        let rbl_b_in_3 = group
            .port_map()
            .port("mux_wl_en_rst_a0")?
            .largest_rect(m0)?;
        let mut rbl_b_in_3_via = via01.clone();
        rbl_b_in_3_via.align_centers_gridded(rbl_b_in_3.bbox(), grid);
        rbl_b_in_3_via.align_left(rbl_b_in_3.bbox());
        let rbl_b_in_3 = router.expand_to_grid(
            rbl_b_in_3_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(rbl_b_in_3_via)?;
        ctx.draw_rect(m1, rbl_b_in_3);
        router.occupy(m1, rbl_b_in_3, "rbl_b")?;

        let rbl_b_in_4 = group.port_map().port("sae_set_b")?.largest_rect(m0)?;
        let mut rbl_b_in_4_via = via01.with_orientation(Named::R90);
        rbl_b_in_4_via.align_centers_gridded(rbl_b_in_4.bbox(), grid);
        let rbl_b_in_4 = router.expand_to_grid(
            rbl_b_in_4_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(rbl_b_in_4_via)?;
        ctx.draw_rect(m1, rbl_b_in_4);
        router.occupy(m1, rbl_b_in_4, "rbl_b")?;

        // pc_read_set_buf -> mux_pc_set
        let pc_read_set_out = group
            .port_map()
            .port("pc_read_set_buf_dout")?
            .largest_rect(m0)?;
        let mut pc_read_set_out_via = via01.clone();
        pc_read_set_out_via.align_centers_gridded(pc_read_set_out.bbox(), grid);
        let pc_read_set_out = router.expand_to_grid(
            pc_read_set_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_read_set_out_via)?;
        ctx.draw_rect(m1, pc_read_set_out);
        router.occupy(m1, pc_read_set_out, "pc_read_set")?;

        let pc_read_set_in = group.port_map().port("mux_pc_set_a0")?.largest_rect(m0)?;
        let mut pc_read_set_in_via = via01.clone();
        pc_read_set_in_via.align_centers_gridded(pc_read_set_in.bbox(), grid);
        pc_read_set_in_via.align_left(pc_read_set_in.bbox());
        let pc_read_set_in = router.expand_to_grid(
            pc_read_set_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_read_set_in_via)?;
        ctx.draw_rect(m1, pc_read_set_in);
        router.occupy(m1, pc_read_set_in, "pc_read_set")?;

        // and_sense_en -> sense_en_delay
        let sense_en_set0_out = group.port_map().port("and_sense_en_x")?.largest_rect(m0)?;
        let mut sense_en_set0_out_via = via01.clone();
        sense_en_set0_out_via.align_centers_gridded(sense_en_set0_out, grid);
        sense_en_set0_out_via.align(AlignMode::Bottom, sense_en_set0_out, 200);
        let sense_en_set0_out = router.expand_to_grid(
            sense_en_set0_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(sense_en_set0_out_via)?;
        ctx.draw_rect(m1, sense_en_set0_out);
        router.occupy(m1, sense_en_set0_out, "sense_en_set0")?;

        let sense_en_set0_in = group
            .port_map()
            .port("sense_en_delay_din")?
            .largest_rect(m0)?;
        let mut sense_en_set0_in_via = via01.clone();
        sense_en_set0_in_via.align_centers_gridded(sense_en_set0_in.bbox(), grid);
        let sense_en_set0_in = router.expand_to_grid(
            sense_en_set0_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(sense_en_set0_in_via)?;
        ctx.draw_rect(m1, sense_en_set0_in);
        router.occupy(m1, sense_en_set0_in, "sense_en_set0")?;

        // sense_en_delay -> sae_ctl
        let sense_en_set_out = group
            .port_map()
            .port("sense_en_delay_dout")?
            .largest_rect(m0)?;
        let mut sense_en_set_out_via = via01.clone();
        sense_en_set_out_via.align_centers_gridded(sense_en_set_out.bbox(), grid);
        let sense_en_set_out = router.expand_to_grid(
            sense_en_set_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(sense_en_set_out_via)?;
        ctx.draw_rect(m1, sense_en_set_out);
        router.occupy(m1, sense_en_set_out, "sense_en_set")?;

        let sense_en_set_in = group.port_map().port("sae_ctl_s")?.largest_rect(m0)?;
        let mut sense_en_set_in_via = via01.with_orientation(Named::R90);
        sense_en_set_in_via.align_centers_gridded(sense_en_set_in.bbox(), grid);
        sense_en_set_in_via.align_left(sense_en_set_in.bbox());
        let sense_en_set_in = router.expand_to_grid(
            sense_en_set_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(sense_en_set_in_via)?;
        ctx.draw_rect(m1, sense_en_set_in);
        router.occupy(m1, sense_en_set_in, "sense_en_set")?;

        // mux_wl_en_rst -> wl_ctl
        let wl_en_rst_out = group.port_map().port("mux_wl_en_rst_x")?.largest_rect(m0)?;
        let mut wl_en_rst_out_via = via01.clone();
        wl_en_rst_out_via.align_centers_gridded(wl_en_rst_out.bbox(), grid);
        let wl_en_rst_out = router.expand_to_grid(
            wl_en_rst_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en_rst_out_via)?;
        ctx.draw_rect(m1, wl_en_rst_out);
        router.occupy(m1, wl_en_rst_out, "wl_en_rst")?;

        let wl_en_rst_in = group.port_map().port("wl_ctl_r")?.largest_rect(m0)?;
        let mut wl_en_rst_in_via = via01.with_orientation(Named::R90);
        wl_en_rst_in_via.align_centers_gridded(wl_en_rst_in.bbox(), grid);
        wl_en_rst_in_via.align_left(wl_en_rst_in.bbox());
        let wl_en_rst_in = router.expand_to_grid(
            wl_en_rst_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en_rst_in_via)?;
        ctx.draw_rect(m1, wl_en_rst_in);
        router.occupy(m1, wl_en_rst_in, "wl_en_rst")?;

        // wl_ctl -> wl_en_buf
        let wl_en0_out = group.port_map().port("wl_ctl_q")?.largest_rect(m0)?;
        let mut wl_en0_out_via = via12.with_orientation(Named::R90);
        wl_en0_out_via.align_centers_gridded(wl_en0_out.bbox(), grid);
        wl_en0_out_via.align_top(wl_en0_out.bbox());
        let wl_en0_out = router.expand_to_grid(
            wl_en0_out_via.layer_bbox(m2).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en0_out_via)?;
        ctx.draw_rect(m2, wl_en0_out);
        router.occupy(m2, wl_en0_out, "wl_en0")?;

        let wl_en0_in = group.port_map().port("wl_en_buf_a")?.largest_rect(m0)?;
        let mut wl_en0_in_via = via01.with_orientation(Named::R90);
        wl_en0_in_via.align_centers_gridded(wl_en0_in.bbox(), grid);
        let wl_en0_in = router.expand_to_grid(
            wl_en0_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en0_in_via)?;
        ctx.draw_rect(m1, wl_en0_in);
        router.occupy(m1, wl_en0_in, "wl_en0")?;

        // sae_ctl -> sae_buf/sae_buf2
        let sense_en0_out = group.port_map().port("sae_ctl_q")?.largest_rect(m0)?;
        let mut sense_en0_out_via = via12.with_orientation(Named::R90);
        sense_en0_out_via.align_centers_gridded(sense_en0_out.bbox(), grid);
        sense_en0_out_via.align_top(sense_en0_out.bbox());
        let sense_en0_out = router.expand_to_grid(
            sense_en0_out_via.layer_bbox(m2).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(sense_en0_out_via)?;
        ctx.draw_rect(m2, sense_en0_out);

        let sense_en0_in_1 = group.port_map().port("sae_buf_a")?.largest_rect(m0)?;
        let mut sense_en0_in_1_via = via01.with_orientation(Named::R90);
        sense_en0_in_1_via.align_centers_gridded(sense_en0_in_1.bbox(), grid);
        let sense_en0_in_1 = router.expand_to_grid(
            sense_en0_in_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(sense_en0_in_1_via)?;
        ctx.draw_rect(m1, sense_en0_in_1);

        let sense_en0_in_2 = group.port_map().port("sae_buf2_a")?.largest_rect(m0)?;
        let mut sense_en0_in_2_via = via01.with_orientation(Named::R90);
        sense_en0_in_2_via.align_centers_gridded(sense_en0_in_2.bbox(), grid);
        let sense_en0_in_2 = router.expand_to_grid(
            sense_en0_in_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(sense_en0_in_2_via)?;
        ctx.draw_rect(m1, sense_en0_in_2);
        let (sense_en0_out_net, sense_en0_in_net) = match self.0 {
            ControlLogicKind::Standard => ("sense_en0", "sense_en0"),
            ControlLogicKind::Test => ("sae_int", "sae_muxed"),
        };
        router.occupy(m2, sense_en0_out, sense_en0_out_net)?;
        router.occupy(m1, sense_en0_in_1, sense_en0_in_net)?;
        router.occupy(m1, sense_en0_in_2, sense_en0_in_net)?;

        // mux_pc_set -> pc_ctl
        let pc_set_out = group.port_map().port("mux_pc_set_x")?.largest_rect(m0)?;
        let mut pc_set_out_via = via01.clone();
        pc_set_out_via.align_centers_gridded(pc_set_out.bbox(), grid);
        let pc_set_out = router.expand_to_grid(
            pc_set_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_set_out_via)?;
        ctx.draw_rect(m1, pc_set_out);
        router.occupy(m1, pc_set_out, "pc_set")?;

        let pc_set_in = group.port_map().port("pc_ctl_s")?.largest_rect(m0)?;
        let mut pc_set_in_via = via01.with_orientation(Named::R90);
        pc_set_in_via.align_centers_gridded(pc_set_in.bbox(), grid);
        pc_set_in_via.align_left(pc_set_in.bbox());
        let pc_set_in = router.expand_to_grid(
            pc_set_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_set_in_via)?;
        ctx.draw_rect(m1, pc_set_in);
        router.occupy(m1, pc_set_in, "pc_set")?;

        // pc_write_set_buf -> mux_pc_set
        let pc_write_set_out = group
            .port_map()
            .port("pc_write_set_buf_dout")?
            .largest_rect(m0)?;
        let mut pc_write_set_out_via = via01.clone();
        pc_write_set_out_via.align_centers_gridded(pc_write_set_out.bbox(), grid);
        let pc_write_set_out = router.expand_to_grid(
            pc_write_set_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_write_set_out_via)?;
        ctx.draw_rect(m1, pc_write_set_out);
        router.occupy(m1, pc_write_set_out, "pc_write_set")?;

        let pc_write_set_in = group.port_map().port("mux_pc_set_a1")?.largest_rect(m0)?;
        let mut pc_write_set_in_via = via01.clone();
        pc_write_set_in_via.align_centers_gridded(pc_write_set_in.bbox(), grid);
        pc_write_set_in_via.align_top(pc_write_set_in.bbox());
        let pc_write_set_in = router.expand_to_grid(
            pc_write_set_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_write_set_in_via)?;
        ctx.draw_rect(m1, pc_write_set_in);
        router.occupy(m1, pc_write_set_in, "pc_write_set")?;

        // pc_ctl -> pc_b_buf/pc_b_buf2
        let pc_b0_out = group.port_map().port("pc_ctl_q_b")?.largest_rect(m0)?;
        let mut pc_b0_out_via = via12.with_orientation(Named::R90);
        pc_b0_out_via.align_centers_gridded(pc_b0_out.bbox(), grid);
        pc_b0_out_via.align_bottom(pc_b0_out.bbox());
        let pc_b0_out = router.expand_to_grid(
            pc_b0_out_via.layer_bbox(m2).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_b0_out_via)?;
        ctx.draw_rect(m2, pc_b0_out);
        router.occupy(m2, pc_b0_out, "pc_b0")?;

        let pc_b0_in_1 = group.port_map().port("pc_b_buf_a")?.largest_rect(m0)?;
        let mut pc_b0_in_1_via = via01.with_orientation(Named::R90);
        pc_b0_in_1_via.align_centers_gridded(pc_b0_in_1.bbox(), grid);
        let pc_b0_in_1 = router.expand_to_grid(
            pc_b0_in_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_b0_in_1_via)?;
        ctx.draw_rect(m1, pc_b0_in_1);
        router.occupy(m1, pc_b0_in_1, "pc_b0")?;

        let pc_b0_in_2 = group.port_map().port("pc_b_buf2_a")?.largest_rect(m0)?;
        let mut pc_b0_in_2_via = via01.with_orientation(Named::R90);
        pc_b0_in_2_via.align_centers_gridded(pc_b0_in_2.bbox(), grid);
        let pc_b0_in_2 = router.expand_to_grid(
            pc_b0_in_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(pc_b0_in_2_via)?;
        ctx.draw_rect(m1, pc_b0_in_2);
        router.occupy(m1, pc_b0_in_2, "pc_b0")?;

        // wr_drv_set_decoder_delay_replica -> wr_drv_ctl
        let wr_drv_set_out = group
            .port_map()
            .port("wr_drv_set_decoder_delay_replica_dout")?
            .largest_rect(m0)?;
        let mut wr_drv_set_out_via = via01.clone();
        wr_drv_set_out_via.align_centers_gridded(wr_drv_set_out.bbox(), grid);
        let wr_drv_set_out = router.expand_to_grid(
            wr_drv_set_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wr_drv_set_out_via)?;
        ctx.draw_rect(m1, wr_drv_set_out);
        router.occupy(m1, wr_drv_set_out, "wr_drv_set")?;

        let wr_drv_set_in = group.port_map().port("wr_drv_ctl_s")?.largest_rect(m0)?;
        let mut wr_drv_set_in_via = via01.with_orientation(Named::R90);
        wr_drv_set_in_via.align_centers_gridded(wr_drv_set_in.bbox(), grid);
        wr_drv_set_in_via.align_left(wr_drv_set_in.bbox());
        let wr_drv_set_in = router.expand_to_grid(
            wr_drv_set_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wr_drv_set_in_via)?;
        ctx.draw_rect(m1, wr_drv_set_in);
        router.occupy(m1, wr_drv_set_in, "wr_drv_set")?;

        // wr_drv_set -> wr_drv_set_decoder_delay_replica
        let wr_drv_set_undelayed_out = group.port_map().port("wr_drv_set_x")?.largest_rect(m0)?;
        let mut wr_drv_set_undelayed_out_via = via01.clone();
        wr_drv_set_undelayed_out_via.align_centers_gridded(wr_drv_set_undelayed_out.bbox(), grid);
        let wr_drv_set_undelayed_out = router.expand_to_grid(
            wr_drv_set_undelayed_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wr_drv_set_undelayed_out_via)?;
        ctx.draw_rect(m1, wr_drv_set_undelayed_out);
        router.occupy(m1, wr_drv_set_undelayed_out, "wr_drv_set_undelayed")?;

        let wr_drv_set_undelayed_in = group
            .port_map()
            .port("wr_drv_set_decoder_delay_replica_din")?
            .largest_rect(m0)?;
        let mut wr_drv_set_undelayed_in_via = via01.with_orientation(Named::R90);
        wr_drv_set_undelayed_in_via.align_centers_gridded(wr_drv_set_undelayed_in.bbox(), grid);
        let wr_drv_set_undelayed_in = router.expand_to_grid(
            wr_drv_set_undelayed_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wr_drv_set_undelayed_in_via)?;
        ctx.draw_rect(m1, wr_drv_set_undelayed_in);
        router.occupy(m1, wr_drv_set_undelayed_in, "wr_drv_set_undelayed")?;

        // wl_en_write_rst_buf -> pc_write_set_buf/mux_wl_en_rst/wr_drv_ctl
        let wl_en_write_rst_out = group
            .port_map()
            .port("wl_en_write_rst_buf_dout")?
            .largest_rect(m0)?;
        let mut wl_en_write_rst_out_via = via01.clone();
        wl_en_write_rst_out_via.align_centers_gridded(wl_en_write_rst_out.bbox(), grid);
        let wl_en_write_rst_out = router.expand_to_grid(
            wl_en_write_rst_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en_write_rst_out_via)?;
        ctx.draw_rect(m1, wl_en_write_rst_out);
        router.occupy(m1, wl_en_write_rst_out, "wl_en_write_rst")?;

        let wl_en_write_rst_in_1 = group
            .port_map()
            .port("pc_write_set_buf_din")?
            .largest_rect(m0)?;
        let mut wl_en_write_rst_in_1_via = via01.with_orientation(Named::R90);
        wl_en_write_rst_in_1_via.align_centers_gridded(wl_en_write_rst_in_1.bbox(), grid);
        let wl_en_write_rst_in_1 = router.expand_to_grid(
            wl_en_write_rst_in_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en_write_rst_in_1_via)?;
        ctx.draw_rect(m1, wl_en_write_rst_in_1);
        router.occupy(m1, wl_en_write_rst_in_1, "wl_en_write_rst")?;

        let wl_en_write_rst_in_2 = group
            .port_map()
            .port("mux_wl_en_rst_a1")?
            .largest_rect(m0)?;
        let mut wl_en_write_rst_in_2_via = via01.clone();
        wl_en_write_rst_in_2_via.align_centers_gridded(wl_en_write_rst_in_2.bbox(), grid);
        wl_en_write_rst_in_2_via.align_top(wl_en_write_rst_in_2.bbox());
        let wl_en_write_rst_in_2 = router.expand_to_grid(
            wl_en_write_rst_in_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en_write_rst_in_2_via)?;
        ctx.draw_rect(m1, wl_en_write_rst_in_2);
        router.occupy(m1, wl_en_write_rst_in_2, "wl_en_write_rst")?;

        let wl_en_write_rst_in_3 = group.port_map().port("wr_drv_ctl_r")?.largest_rect(m0)?;
        let mut wl_en_write_rst_in_3_via = via01.with_orientation(Named::R90);
        wl_en_write_rst_in_3_via.align_centers_gridded(wl_en_write_rst_in_3.bbox(), grid);
        wl_en_write_rst_in_3_via.align_left(wl_en_write_rst_in_3.bbox());
        let wl_en_write_rst_in_3 = router.expand_to_grid(
            wl_en_write_rst_in_3_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en_write_rst_in_3_via)?;
        ctx.draw_rect(m1, wl_en_write_rst_in_3);
        router.occupy(m1, wl_en_write_rst_in_3, "wl_en_write_rst")?;

        // wr_drv_ctl -> wr_drv_buf
        let write_driver_en0_out = group.port_map().port("wr_drv_ctl_q")?.largest_rect(m0)?;
        let mut write_driver_en0_out_via = via12.with_orientation(Named::R90);
        write_driver_en0_out_via.align_centers_gridded(write_driver_en0_out.bbox(), grid);
        write_driver_en0_out_via.align_top(write_driver_en0_out.bbox());
        let write_driver_en0_out = router.expand_to_grid(
            write_driver_en0_out_via.layer_bbox(m2).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(write_driver_en0_out_via)?;
        ctx.draw_rect(m2, write_driver_en0_out);
        router.occupy(m2, write_driver_en0_out, "write_driver_en0")?;

        let write_driver_en0_in = group.port_map().port("wr_drv_buf_a")?.largest_rect(m0)?;
        let mut write_driver_en0_in_via = via01.with_orientation(Named::R90);
        write_driver_en0_in_via.align_centers_gridded(write_driver_en0_in.bbox(), grid);
        let write_driver_en0_in = router.expand_to_grid(
            write_driver_en0_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Right),
        );
        ctx.draw(write_driver_en0_in_via)?;
        ctx.draw_rect(m1, write_driver_en0_in);
        router.occupy(m1, write_driver_en0_in, "write_driver_en0")?;

        // clk_pin -> inv_clk
        let clk_in = group.port_map().port("inv_clk_din")?.largest_rect(m0)?;
        let edge = clk_in.edge(Side::Left);
        ctx.draw(
            ElbowJog::builder()
                .src(
                    clk_in
                        .edge(Side::Left)
                        .with_span(edge.span().shrink_all(40)),
                )
                .dst(clk_pin.center())
                .layer(m0)
                .build()
                .unwrap(),
        )?;

        let mut clk_pin_via = via01.with_orientation(Named::R90);
        clk_pin_via.align_centers_gridded(clk_pin, grid);
        ctx.draw(clk_pin_via)?;

        // we_pin -> inv_we/mux_wl_en_rst/mux_pc_set/wr_drv_set
        let we_in_1 = group.port_map().port("inv_we_a")?.largest_rect(m0)?;
        let mut we_in_1_via = via01.with_orientation(Named::R90);
        we_in_1_via.align_centers_gridded(we_in_1.bbox(), grid);
        let we_in_1 = router.expand_to_grid(
            we_in_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(we_in_1_via)?;
        ctx.draw_rect(m1, we_in_1);
        router.occupy(m1, we_in_1, "we")?;

        let we_in_2 = group.port_map().port("mux_wl_en_rst_s")?.largest_rect(m0)?;
        let mut we_in_2_via = via01.clone();
        we_in_2_via.align_centers_gridded(we_in_2.bbox(), grid);
        we_in_2_via.align_bottom(we_in_2.bbox());
        let we_in_2 = router.expand_to_grid(
            we_in_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(we_in_2_via)?;
        ctx.draw_rect(m1, we_in_2);
        router.occupy(m1, we_in_2, "we")?;

        let we_in_3 = group.port_map().port("mux_pc_set_s")?.largest_rect(m0)?;
        let mut we_in_3_via = via01.clone();
        we_in_3_via.align_centers_gridded(we_in_3.bbox(), grid);
        we_in_3_via.align_bottom(we_in_3.bbox());
        let we_in_3 = router.expand_to_grid(
            we_in_3_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(we_in_3_via)?;
        ctx.draw_rect(m1, we_in_3);
        router.occupy(m1, we_in_3, "we")?;

        let we_in_4 = group.port_map().port("wr_drv_set_b")?.largest_rect(m0)?;
        let mut we_in_4_via = via01.with_orientation(Named::R90);
        we_in_4_via.align_centers_gridded(we_in_4.bbox(), grid);
        let we_in_4 = router.expand_to_grid(
            we_in_4_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(we_in_4_via)?;
        ctx.draw_rect(m1, we_in_4);
        router.occupy(m1, we_in_4, "we")?;

        // pc_b_buf/pc_b_buf2 -> pc_b_pin
        let pc_b_out_1 = group.port_map().port("pc_b_buf_x")?.largest_rect(m0)?;
        let mut pc_b_out_1_via = via01.clone();
        pc_b_out_1_via.align_centers_gridded(pc_b_out_1.bbox(), grid);
        let pc_b_out_1 = router.expand_to_grid(
            pc_b_out_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Left),
        );
        ctx.draw(pc_b_out_1_via)?;
        ctx.draw_rect(m1, pc_b_out_1);
        router.occupy(m1, pc_b_out_1, "pc_b")?;

        let pc_b_out_2 = group.port_map().port("pc_b_buf2_x")?.largest_rect(m0)?;
        let mut pc_b_out_2_via = via01.clone();
        pc_b_out_2_via.align_centers_gridded(pc_b_out_2.bbox(), grid);
        let pc_b_out_2 = router.expand_to_grid(
            pc_b_out_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::LowerLeft),
        );
        ctx.draw(pc_b_out_2_via)?;
        ctx.draw_rect(m1, pc_b_out_2);
        // router.occupy(m1, pc_b_out_2, "pc_b")?;
        router.block(m1, pc_b_out_2);

        // wl_ctl -> wl_en0_pin
        let wl_en0_out = group.port_map().port("wl_ctl_q")?.largest_rect(m0)?;
        let mut wl_en0_out_via = via12.with_orientation(Named::R90);
        wl_en0_out_via.align_centers_gridded(wl_en0_out.bbox(), grid);
        wl_en0_out_via.align_top(wl_en0_out.bbox());
        let wl_en0_out = router.expand_to_grid(
            wl_en0_out_via.layer_bbox(m2).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wl_en0_out_via)?;
        ctx.draw_rect(m2, wl_en0_out);
        router.occupy(m2, wl_en0_out, "wl_en0")?;

        // wl_en_buf -> wl_en_pin/wbl_pulldown_en
        let wl_en_out = group.port_map().port("wl_en_buf_x")?.largest_rect(m0)?;
        let mut wl_en_out_via = via01.clone();
        wl_en_out_via.align_centers_gridded(wl_en_out.bbox(), grid);
        let wl_en_out = router.expand_to_grid(
            wl_en_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Left),
        );
        ctx.draw(wl_en_out_via)?;
        ctx.draw_rect(m1, wl_en_out);
        // router.occupy(m1, wl_en_out, "wl_en")?;
        router.block(m1, wl_en_out);

        let wl_en_in = group
            .port_map()
            .port("wbl_pulldown_en_a")?
            .largest_rect(m0)?;
        let mut wl_en_in_via = via01.with_orientation(Named::R90);
        wl_en_in_via.align_centers_gridded(wl_en_in.bbox(), grid);
        wl_en_in_via.align_top(wl_en_in.bbox());
        let wl_en_in = router.expand_to_grid(
            wl_en_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Top),
        );
        ctx.draw(wl_en_in_via)?;
        ctx.draw_rect(m1, wl_en_in);
        router.occupy(m1, wl_en_in, "wl_en")?;

        // wr_drv_buf -> write_driver_en_pin/wbl_pulldown_en
        let write_driver_en_out = group.port_map().port("wr_drv_buf_x")?.largest_rect(m0)?;
        let mut write_driver_en_out_via = via01.clone();
        write_driver_en_out_via.align_centers_gridded(write_driver_en_out.bbox(), grid);
        let write_driver_en_out = router.expand_to_grid(
            write_driver_en_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(write_driver_en_out_via)?;
        ctx.draw_rect(m1, write_driver_en_out);
        router.occupy(m1, write_driver_en_out, "write_driver_en")?;

        let write_driver_en_in = group
            .port_map()
            .port("wbl_pulldown_en_b")?
            .largest_rect(m0)?;
        let mut write_driver_en_in_via = via01.with_orientation(Named::R90);
        write_driver_en_in_via.align_centers_gridded(write_driver_en_in.bbox(), grid);
        let write_driver_en_in = router.expand_to_grid(
            write_driver_en_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(write_driver_en_in_via)?;
        ctx.draw_rect(m1, write_driver_en_in);
        router.occupy(m1, write_driver_en_in, "write_driver_en")?;

        // sae_buf/sae_buf2 -> sense_en_pin
        let sense_en_out_1 = group.port_map().port("sae_buf_x")?.largest_rect(m0)?;
        let mut sense_en_out_1_via = via01.clone();
        sense_en_out_1_via.align_centers_gridded(sense_en_out_1.bbox(), grid);
        let sense_en_out_1 = router.expand_to_grid(
            sense_en_out_1_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Left),
        );
        ctx.draw(sense_en_out_1_via)?;
        ctx.draw_rect(m1, sense_en_out_1);
        router.occupy(m1, sense_en_out_1, "sense_en")?;

        let sense_en_out_2 = group.port_map().port("sae_buf2_x")?.largest_rect(m0)?;
        let mut sense_en_out_2_via = via01.clone();
        sense_en_out_2_via.align_centers_gridded(sense_en_out_2.bbox(), grid);
        let sense_en_out_2 = router.expand_to_grid(
            sense_en_out_2_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperLeft),
        );
        ctx.draw(sense_en_out_2_via)?;
        ctx.draw_rect(m1, sense_en_out_2);
        // router.occupy(m1, sense_en_out_2, "sense_en")?;
        router.block(m1, sense_en_out_2);

        // rbl_pin -> inv_rbl
        let rbl_in = group.port_map().port("inv_rbl_a")?.largest_rect(m0)?;
        let mut rbl_in_via = via01.with_orientation(Named::R90);
        rbl_in_via.align_centers_gridded(rbl_in.bbox(), grid);
        let rbl_in = router.expand_to_grid(
            rbl_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Left),
        );
        ctx.draw(rbl_in_via)?;
        ctx.draw_rect(m1, rbl_in);
        // router.occupy(m1, rbl_in, "rbl")?;
        router.block(m1, rbl_in);

        // dummy_bl_pin -> inv_dummy_bl
        let dummy_bl_in = group
            .port_map()
            .port("wl_en_write_rst_buf_din")?
            .largest_rect(m0)?;
        let mut dummy_bl_in_via = via01.with_orientation(Named::R90);
        dummy_bl_in_via.align_centers_gridded(dummy_bl_in.bbox(), grid);
        let dummy_bl_in = router.expand_to_grid(
            dummy_bl_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Corner(Corner::UpperRight),
        );
        ctx.draw(dummy_bl_in_via)?;
        ctx.draw_rect(m1, dummy_bl_in);
        router.occupy(m1, dummy_bl_in, "dummy_bl")?;

        // wbl_pulldown_en -> dummy_bl_pulldown
        let wbl_pulldown_en_out = group
            .port_map()
            .port("wbl_pulldown_en_x")?
            .largest_rect(m0)?;
        let mut wbl_pulldown_en_out_via = via01.clone();
        wbl_pulldown_en_out_via.align_centers_gridded(wbl_pulldown_en_out.bbox(), grid);
        let wbl_pulldown_en_out = router.expand_to_grid(
            wbl_pulldown_en_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wbl_pulldown_en_out_via)?;
        ctx.draw_rect(m1, wbl_pulldown_en_out);
        router.occupy(m1, wbl_pulldown_en_out, "wbl_pulldown_en")?;

        let wbl_pulldown_en_in = group
            .port_map()
            .port("dummy_bl_pulldown_gate_0")?
            .largest_rect(m0)?;
        let mut wbl_pulldown_en_in_via = via01.with_orientation(Named::R90);
        wbl_pulldown_en_in_via.align_centers_gridded(wbl_pulldown_en_in.bbox(), grid);
        let wbl_pulldown_en_in = router.expand_to_grid(
            wbl_pulldown_en_in_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Minimum,
        );
        ctx.draw(wbl_pulldown_en_in_via)?;
        ctx.draw_rect(m1, wbl_pulldown_en_in);
        router.occupy(m1, wbl_pulldown_en_in, "wbl_pulldown_en")?;

        // dummy_bl_pulldown -> dummy_bl_pin
        let dummy_bl_out = group
            .port_map()
            .port("dummy_bl_pulldown_sd_0_0")?
            .largest_rect(m0)?;
        let mut dummy_bl_out_via = via01.with_orientation(Named::R90);
        dummy_bl_out_via.align_centers_gridded(dummy_bl_out.bbox(), grid);
        let dummy_bl_out = router.expand_to_grid(
            dummy_bl_out_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Bot),
        );
        ctx.draw(dummy_bl_out_via)?;
        ctx.draw_rect(m1, dummy_bl_out);
        router.occupy(m1, dummy_bl_out, "dummy_bl")?;

        // dummy_bl_pulldown -> vss
        let dummy_bl_vss = group
            .port_map()
            .port("dummy_bl_pulldown_sd_0_1")?
            .largest_rect(m0)?;
        let mut dummy_bl_vss_via = via01.with_orientation(Named::R90);
        dummy_bl_vss_via.align_centers_gridded(dummy_bl_vss.bbox(), grid);
        let dummy_bl_vss = router.expand_to_grid(
            dummy_bl_vss_via.layer_bbox(m1).into_rect(),
            ExpandToGridStrategy::Side(Side::Top),
        );
        ctx.draw(dummy_bl_vss_via)?;
        ctx.draw_rect(m1, dummy_bl_vss);
        router.occupy(m1, dummy_bl_vss, "vss")?;

        router.route_with_net(ctx, m1, clkp_out, m1, clkp_in_1, "clkp")?;
        router.route_with_net(ctx, m1, clkp_out, m1, clkp_in_2, "clkp")?;
        router.route_with_net(ctx, m1, clkp_out, m1, clkp_in_3, "clkp")?;
        router.route_with_net(ctx, m1, wl_en_set_out, m1, wl_en_set_in, "wl_en_set")?;
        router.route_with_net(ctx, m1, we_b_out, m1, we_b_in_1, "we_b")?;
        router.route_with_net(ctx, m1, we_b_out, m1, we_b_in_2, "we_b")?;
        router.route_with_net(ctx, m1, rbl_b_out, m1, rbl_b_in_2, "rbl_b")?;
        router.route_with_net(ctx, m1, rbl_b_out, m1, rbl_b_in_3, "rbl_b")?;
        router.route_with_net(ctx, m1, rbl_b_out, m1, rbl_b_in_4, "rbl_b")?;
        router.route_with_net(ctx, m1, pc_read_set_out, m1, pc_read_set_in, "pc_read_set")?;
        router.route_with_net(
            ctx,
            m1,
            sense_en_set0_out,
            m1,
            sense_en_set0_in,
            "sense_en_set0",
        )?;
        router.route_with_net(
            ctx,
            m1,
            sense_en_set_out,
            m1,
            sense_en_set_in,
            "sense_en_set",
        )?;
        router.route_with_net(ctx, m1, wl_en_rst_out, m1, wl_en_rst_in, "wl_en_rst")?;
        router.route_with_net(ctx, m2, wl_en0_out, m1, wl_en0_in, "wl_en0")?;
        match self.0 {
            ControlLogicKind::Standard => {
                router.route_with_net(ctx, m2, sense_en0_out, m1, sense_en0_in_1, "sense_en0")?;
                router.route_with_net(ctx, m2, sense_en0_out, m1, sense_en0_in_2, "sense_en0")?;
            }
            ControlLogicKind::Test => {
                router.route_with_net(ctx, m2, sense_en0_out, m1, sae_int, "sae_int")?;
                router.route_with_net(ctx, m1, sae_muxed, m1, sense_en0_in_1, "sae_muxed")?;
                router.route_with_net(ctx, m1, sae_muxed, m1, sense_en0_in_2, "sae_muxed")?;
            }
        }
        router.route_with_net(ctx, m1, pc_set_out, m1, pc_set_in, "pc_set")?;
        router.route_with_net(
            ctx,
            m1,
            pc_write_set_out,
            m1,
            pc_write_set_in,
            "pc_write_set",
        )?;
        router.route_with_net(ctx, m2, pc_b0_out, m1, pc_b0_in_1, "pc_b0")?;
        router.route_with_net(ctx, m2, pc_b0_out, m1, pc_b0_in_2, "pc_b0")?;
        router.route_with_net(ctx, m1, wr_drv_set_out, m1, wr_drv_set_in, "wr_drv_set")?;
        router.route_with_net(
            ctx,
            m1,
            wr_drv_set_undelayed_out,
            m1,
            wr_drv_set_undelayed_in,
            "wr_drv_set_undelayed",
        )?;
        router.route_with_net(
            ctx,
            m1,
            wl_en_write_rst_out,
            m1,
            wl_en_write_rst_in_1,
            "wl_en_write_rst",
        )?;
        router.route_with_net(
            ctx,
            m1,
            wl_en_write_rst_out,
            m1,
            wl_en_write_rst_in_2,
            "wl_en_write_rst",
        )?;
        router.route_with_net(
            ctx,
            m1,
            wl_en_write_rst_out,
            m1,
            wl_en_write_rst_in_3,
            "wl_en_write_rst",
        )?;
        router.route_with_net(
            ctx,
            m2,
            write_driver_en0_out,
            m1,
            write_driver_en0_in,
            "write_driver_en0",
        )?;
        router.route_with_net(ctx, m1, sense_en_out_1, m1, sense_en_pin, "sense_en")?;
        router.route_with_net(ctx, m1, sense_en_out_2, m1, sense_en_pin, "sense_en")?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in_1, "we")?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in_2, "we")?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in_3, "we")?;
        router.route_with_net(ctx, m1, we_pin, m1, we_in_4, "we")?;
        router.route_with_net(ctx, m1, pc_b_out_1, m1, pc_b_pin, "pc_b")?;
        router.route_with_net(ctx, m1, pc_b_out_2, m1, pc_b_pin, "pc_b")?;
        router.route_with_net(ctx, m2, wl_en0_out, m1, wl_en0_pin, "wl_en0")?;
        router.route_with_net(ctx, m1, wl_en_out, m1, wl_en_in, "wl_en")?;
        router.route_with_net(ctx, m1, wl_en_out, m1, wl_en_pin, "wl_en")?;
        router.route_with_net(
            ctx,
            m1,
            write_driver_en_out,
            m1,
            write_driver_en_in,
            "write_driver_en",
        )?;
        router.route_with_net(
            ctx,
            m1,
            write_driver_en_out,
            m1,
            write_driver_en_pin,
            "write_driver_en",
        )?;
        router.route_with_net(
            ctx,
            m1,
            wbl_pulldown_en_out,
            m1,
            wbl_pulldown_en_in,
            "wbl_pulldown_en",
        )?;
        router.route_with_net(ctx, m1, dummy_bl_in, m1, dummy_bl_pin, "dummy_bl")?;
        router.route_with_net(ctx, m1, dummy_bl_out, m1, dummy_bl_pin, "dummy_bl")?;
        router.route_with_net(ctx, m1, vss_rect, m1, dummy_bl_vss, "vss")?;
        router.route_with_net(ctx, m1, rbl_in, m1, rbl_pin, "rbl")?;

        ctx.draw(router)?;

        ctx.add_port(CellPort::with_shape("clk", m1, clk_pin))?;
        ctx.add_port(CellPort::with_shape("we", m1, we_pin))?;
        if matches!(self.0, ControlLogicKind::Test) {
            ctx.add_port(CellPort::with_shape("sae_int", m1, sae_int))?;
            ctx.add_port(CellPort::with_shape("sae_muxed", m1, sae_muxed))?;
        }
        ctx.add_port(CellPort::with_shape("pc_b", m1, pc_b_pin))?;
        ctx.add_port(CellPort::with_shape("wl_en0", m1, wl_en0_pin))?;
        ctx.add_port(CellPort::with_shape("wl_en", m1, wl_en_pin))?;
        ctx.add_port(CellPort::with_shape(
            "write_driver_en",
            m1,
            write_driver_en_pin,
        ))?;
        ctx.add_port(CellPort::with_shape("sense_en", m1, sense_en_pin))?;
        ctx.add_port(CellPort::with_shape("rbl", m1, rbl_pin))?;
        ctx.add_port(CellPort::with_shape("dummy_bl", m1, dummy_bl_pin))?;

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
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;
        let tap = lib.try_cell_named("sky130_fd_sc_hd__tap_2")?;
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
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;
        let nor = lib.try_cell_named("sky130_fd_sc_hd__nor2_2")?;
        let nor = ctx.instantiate::<StdCell>(&nor.id())?;
        let nor_hflip = nor.with_orientation(Named::ReflectHoriz);

        let layers = ctx.inner().layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let grid = ctx.pdk().layout_grid();

        let nor = LayerBbox::new(nor, outline);
        let nor_hflip = LayerBbox::new(nor_hflip, outline);
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

        row.push(nor);
        row.push(nor_hflip);

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

        via.align_centers_gridded(b0.largest_rect(m0)?, grid);
        via.align_left(b0.largest_rect(m0)?);
        let b0_via = via.clone();

        via.align_centers_gridded(y0.largest_rect(m0)?, grid);
        via.align_bottom(y0.largest_rect(m0)?);
        let y0_via = via.clone();

        via.align_centers_gridded(b1.largest_rect(m0)?, grid);
        via.align_left(b1.largest_rect(m0)?);
        let b1_via = via.clone();

        via.align_centers_gridded(y1.largest_rect(m0)?, grid);
        via.align_top(y1.largest_rect(m0)?);
        let y1_via = via.clone();

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
                .src(y1_via.layer_bbox(m1).into_rect().edge(Side::Left))
                .dst(b0_via.brect().center())
                .layer(m1)
                .build()
                .unwrap(),
        )?;

        ctx.draw(b0_via)?;
        ctx.draw(y0_via)?;
        ctx.draw(b1_via)?;
        ctx.draw(y1_via)?;

        ctx.add_port(a0.clone().with_id("r"))?;
        ctx.add_port(y0.clone().with_id("q"))?;
        ctx.add_port(a1.clone().with_id("s"))?;
        ctx.add_port(y1.clone().with_id("q_b"))?;

        let vpwr0 = row.port_map().port(PortId::new("vpwr", 0))?;
        let vgnd0 = row.port_map().port(PortId::new("vgnd", 0))?;
        let vpwr1 = row.port_map().port(PortId::new("vpwr", 1))?;
        let vgnd1 = row.port_map().port(PortId::new("vgnd", 1))?;
        ctx.merge_port(vpwr0.clone().with_id("vdd"));
        ctx.merge_port(vgnd0.clone().with_id("vss"));
        ctx.merge_port(vpwr1.clone().with_id("vdd"));
        ctx.merge_port(vgnd1.clone().with_id("vss"));

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
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;
        let inv = lib.try_cell_named("sky130_fd_sc_hd__inv_2")?;
        let inv = ctx.instantiate::<StdCell>(&inv.id())?;

        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let outline = layers.get(Selector::Name("outline"))?;

        let mut row = new_row();
        row.push_num(LayerBbox::new(inv, outline), self.n);
        let mut row = row.build();
        row.expose_ports(
            |port: CellPort, i| Some(port.with_index(i)),
            PortConflictStrategy::Error,
        )?;

        for i in 0..self.n - 1 {
            let y = row.port_map().port(PortId::new("y", i))?.largest_rect(m0)?;
            let a = row
                .port_map()
                .port(PortId::new("a", i + 1))?
                .largest_rect(m0)?;
            ctx.draw_rect(m0, Rect::from_spans(a.hspan().union(y.hspan()), a.vspan()));
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
        let lib = stdcells.try_lib_named("sky130_fd_sc_hd")?;
        let and = lib.try_cell_named("sky130_fd_sc_hd__and2_4")?;
        let and = ctx.instantiate::<StdCell>(&and.id())?;

        let layers = ctx.layers();
        let outline = layers.get(Selector::Name("outline"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let grid = ctx.pdk().layout_grid();

        let inv_chain = ctx.instantiate::<InvChain>(&9)?;

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

        row.add_rect(m1, dout_via.bbox().union(b_via.bbox()).into_rect());

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
