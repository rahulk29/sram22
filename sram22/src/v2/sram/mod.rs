use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::{Dir, Rect, Span};
use substrate::component::Component;
use substrate::layout::cell::{CellPort, Port, PortId};
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::routing::auto::straps::PlacedStraps;
use substrate::layout::straps::SingleSupplyNet;
#[cfg(test)]
use substrate::schematic::netlist::NetlistPurpose;

use super::guard_ring::{GuardRing, GuardRingParams, SupplyRings};

pub mod layout;
pub mod schematic;
pub mod testbench;
pub mod verilog;

pub struct SramInner {
    params: SramParams,
}

pub struct Sram {
    params: SramParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SramParams {
    pub wmask_width: usize,

    // Schematic
    pub row_bits: usize,
    pub col_bits: usize,
    pub col_select_bits: usize,

    // Layout
    pub rows: usize,
    pub cols: usize,
    pub mux_ratio: usize,

    // Verilog
    pub num_words: usize,
    pub data_width: usize,
    pub addr_width: usize,

    pub control: ControlMode,
}

impl SramParams {
    pub const fn new(
        wmask_granularity: usize,
        mux_ratio: usize,
        num_words: usize,
        data_width: usize,
        control: ControlMode,
    ) -> Self {
        Self {
            wmask_width: data_width / wmask_granularity,
            row_bits: (num_words / mux_ratio).ilog2() as usize,
            col_bits: (data_width * mux_ratio).ilog2() as usize,
            col_select_bits: mux_ratio.ilog2() as usize,
            rows: num_words / mux_ratio,
            cols: data_width * mux_ratio,
            mux_ratio,
            num_words,
            data_width,
            addr_width: num_words.ilog2() as usize,
            control,
        }
    }

    #[inline]
    pub fn wmask_granularity(&self) -> usize {
        self.data_width / self.wmask_width
    }

    /// The name of the SRAM cell with these parameters.
    pub fn name(&self) -> arcstr::ArcStr {
        arcstr::format!(
            "sram22_{}x{}m{}w{}",
            self.num_words,
            self.data_width,
            self.mux_ratio,
            self.wmask_granularity()
        )
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum ControlMode {
    ReplicaV2,
    ReplicaV2Test,
}

impl Component for SramInner {
    type Params = SramParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sram22_inner")
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for Sram {
    type Params = SramParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        self.params.name()
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let mut inner = ctx.instantiate::<SramInner>(&self.params)?;
        ctx.bubble_all_ports(&mut inner);
        ctx.add_instance(inner);
        Ok(())
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let sram = ctx.instantiate::<SramInner>(&self.params)?;
        let brect = sram.brect();
        ctx.draw_ref(&sram)?;

        let m1 = ctx.layers().get(Selector::Metal(1))?;
        let m2 = ctx.layers().get(Selector::Metal(2))?;
        let m3 = ctx.layers().get(Selector::Metal(3))?;
        let params = GuardRingParams {
            enclosure: brect.expand(1_000),
            h_metal: m2,
            v_metal: m1,
            h_width: 1_360,
            v_width: 1_360,
        };
        let ring = ctx.instantiate::<GuardRing>(&params)?;
        let rings = ring.cell().get_metadata::<SupplyRings>();
        let straps = sram.cell().get_metadata::<PlacedStraps>();

        for (layer, dir) in [(m2, Dir::Horiz), (m3, Dir::Vert)] {
            for strap in straps.on_layer(layer) {
                let ring = match strap.net {
                    SingleSupplyNet::Vss => rings.vss,
                    SingleSupplyNet::Vdd => rings.vdd,
                };
                assert_ne!(strap.rect.area(), 0);
                let lower = if strap.lower_boundary {
                    ring.outer().span(dir).start()
                } else {
                    strap.rect.span(dir).stop()
                };
                let upper = if strap.upper_boundary {
                    ring.outer().span(dir).stop()
                } else {
                    strap.rect.span(dir).start()
                };

                let r = Rect::span_builder()
                    .with(dir, Span::new(lower, upper))
                    .with(!dir, strap.rect.span(!dir))
                    .build();

                let below = if layer == m2 { m1 } else { m2 };

                if strap.upper_boundary {
                    let target = ring.dir_rects(!dir)[1];
                    let viap = ViaParams::builder()
                        .layers(below, layer)
                        .geometry(target, r)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    ctx.instantiate::<Via>(&viap)?.add_to(ctx)?;
                }
                if strap.lower_boundary {
                    let target = ring.dir_rects(!dir)[0];
                    let viap = ViaParams::builder()
                        .layers(below, layer)
                        .geometry(target, r)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    ctx.instantiate::<Via>(&viap)?.add_to(ctx)?;
                }
                ctx.draw_rect(layer, r);
            }
        }
        for port in ["vdd", "vss"] {
            ctx.add_port(
                ring.port(format!("ring_{port}"))?
                    .into_cell_port()
                    .named(port),
            )?;
        }

        ctx.draw(ring)?;

        // Route pins to edge of guard ring
        let groups = self.params.cols / self.params.mux_ratio;
        for (pin, width) in if self.params.control == ControlMode::ReplicaV2 {
            vec![
                ("dout", groups),
                ("din", groups),
                ("wmask", self.params.wmask_width),
                ("addr", self.params.addr_width),
                ("we", 1),
                ("clk", 1),
            ]
        } else {
            vec![
                ("dout", groups),
                ("din", groups),
                ("wmask", self.params.wmask_width),
                ("addr", self.params.addr_width),
                ("we", 1),
                ("clk", 1),
                ("sae_int", 1),
                ("sae_muxed", 1),
            ]
        } {
            for i in 0..width {
                let port_id = PortId::new(pin, i);
                let rect = sram.port(port_id.clone())?.largest_rect(m3)?;
                let rect = rect.with_vspan(
                    rect.vspan()
                        .add_point(ctx.bbox().into_rect().side(subgeom::Side::Bot)),
                );
                ctx.draw_rect(m3, rect);
                ctx.add_port(CellPort::builder().id(port_id).add(m3, rect).build())?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use self::verilog::save_1rw_verilog;
    use crate::paths::{out_gds, out_spice, out_verilog};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    pub(crate) const TINY_SRAM: SramParams = SramParams::new(2, 4, 64, 4, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_1: SramParams = SramParams::new(8, 4, 256, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_2: SramParams = SramParams::new(8, 4, 2048, 64, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_3: SramParams = SramParams::new(8, 4, 64, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_4: SramParams = SramParams::new(32, 4, 64, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_5: SramParams = SramParams::new(8, 4, 512, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_6: SramParams =
        SramParams::new(32, 8, 1024, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_7: SramParams = SramParams::new(8, 8, 1024, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_8: SramParams =
        SramParams::new(32, 8, 1024, 64, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_9: SramParams = SramParams::new(8, 8, 2048, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_10: SramParams =
        SramParams::new(8, 8, 4096, 32, ControlMode::ReplicaV2);

    pub(crate) const PARAMS_11: SramParams = SramParams::new(8, 8, 4096, 8, ControlMode::ReplicaV2);
    pub(crate) const ROCKET_1: SramParams = SramParams::new(8, 4, 512, 64, ControlMode::ReplicaV2);
    pub(crate) const ROCKET_2: SramParams = SramParams::new(24, 4, 64, 24, ControlMode::ReplicaV2);
    pub(crate) const ROCKET_3: SramParams = SramParams::new(32, 4, 512, 32, ControlMode::ReplicaV2);
    pub(crate) const ROCKET_4: SramParams = SramParams::new(8, 8, 4096, 32, ControlMode::ReplicaV2);
    pub(crate) const ROCKET_5: SramParams =
        SramParams::new(32, 8, 1024, 32, ControlMode::ReplicaV2);
    pub(crate) const ROCKET_6: SramParams = SramParams::new(8, 8, 1024, 32, ControlMode::ReplicaV2);

    #[test]
    fn test_sram_tiny() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sram_tiny");

        let spice_path = out_spice(&work_dir, "schematic");
        ctx.write_schematic_to_file::<Sram>(&TINY_SRAM, &spice_path)
            .expect("failed to write schematic");

        let gds_path = out_gds(&work_dir, "layout");
        ctx.write_layout::<Sram>(&TINY_SRAM, &gds_path)
            .expect("failed to write layout");

        let verilog_path = out_verilog(&work_dir, "behavioral");
        save_1rw_verilog(&verilog_path, &*TINY_SRAM.name(), &TINY_SRAM)
            .expect("failed to write behavioral model");

        #[cfg(feature = "commercial")]
        {
            crate::abs::run_abstract(
                &work_dir,
                &TINY_SRAM.name(),
                crate::paths::out_lef(&work_dir, "abstract"),
                &gds_path,
                &verilog_path,
            )
            .expect("failed to write abstract");

            let timing_spice_path = out_spice(&work_dir, "timing_schematic");
            ctx.write_schematic_to_file_for_purpose::<Sram>(
                &TINY_SRAM,
                &timing_spice_path,
                NetlistPurpose::Timing,
            )
            .expect("failed to write timing schematic");

            let params = liberate_mx::LibParams::builder()
                .work_dir(work_dir.join("lib"))
                .output_file(crate::paths::out_lib(
                    &work_dir,
                    "timing_tt_025C_1v80.schematic",
                ))
                .corner("tt")
                .cell_name(&*TINY_SRAM.name())
                .num_words(TINY_SRAM.num_words)
                .data_width(TINY_SRAM.data_width)
                .addr_width(TINY_SRAM.addr_width)
                .wmask_width(TINY_SRAM.wmask_width)
                .mux_ratio(TINY_SRAM.mux_ratio)
                .has_wmask(true)
                .source_paths(vec![timing_spice_path])
                .build()
                .unwrap();
            crate::liberate::generate_sram_lib(&params).expect("failed to write lib");
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<Sram>(&TINY_SRAM, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                substrate::verification::drc::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<Sram>(&TINY_SRAM, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                substrate::verification::lvs::LvsSummary::Pass
            ));
        }
    }

    macro_rules! test_sram {
        ($name: ident, $params: ident $(, $attr: meta)*) => {
            #[test]
            $(#[$attr])*
            fn $name() {
                let ctx = setup_ctx();
                let work_dir = test_work_dir(stringify!($name));

                let spice_path = out_spice(&work_dir, "schematic");
                ctx.write_schematic_to_file::<Sram>(&$params, &spice_path)
                    .expect("failed to write schematic");

                let gds_path = out_gds(&work_dir, "layout");
                ctx.write_layout::<Sram>(&$params, &gds_path)
                    .expect("failed to write layout");

                let verilog_path = out_verilog(&work_dir, "behavioral");
                save_1rw_verilog(&verilog_path,&*$params.name(), &$params)
                    .expect("failed to write behavioral model");

                #[cfg(feature = "commercial")]
                {
                    let drc_work_dir = work_dir.join("drc");
                    let output = ctx
                        .write_drc::<Sram>(&$params, drc_work_dir)
                        .expect("failed to run DRC");
                    assert!(matches!(
                        output.summary,
                        substrate::verification::drc::DrcSummary::Pass
                    ));

                    let lvs_work_dir = work_dir.join("lvs");
                    let output = ctx
                        .write_lvs::<Sram>(&$params, lvs_work_dir)
                        .expect("failed to run LVS");
                    assert!(matches!(
                        output.summary,
                        substrate::verification::lvs::LvsSummary::Pass
                    ));

                    crate::abs::run_abstract(
                        &work_dir,
                        &$params.name(),
                        crate::paths::out_lef(&work_dir, "abstract"),
                        &gds_path,
                        &verilog_path,
                    )
                    .expect("failed to write abstract");

                    let timing_spice_path = out_spice(&work_dir, "timing_schematic");
                    ctx.write_schematic_to_file_for_purpose::<Sram>(
                        &TINY_SRAM,
                        &timing_spice_path,
                        NetlistPurpose::Timing,
                    )
                    .expect("failed to write timing schematic");

                    let params = liberate_mx::LibParams::builder()
                        .work_dir(work_dir.join("lib"))
                        .output_file(crate::paths::out_lib(&work_dir, "timing_tt_025C_1v80.schematic"))
                        .corner("tt")
                        .cell_name(&*$params.name())
                        .num_words($params.num_words)
                        .data_width($params.data_width)
                        .addr_width($params.addr_width)
                        .wmask_width($params.wmask_width)
                        .mux_ratio($params.mux_ratio)
                        .has_wmask(true)
                        .source_paths(vec![timing_spice_path])
                        .build()
                        .unwrap();
                    crate::liberate::generate_sram_lib(&params).expect("failed to write lib");
                }
            }
        };
    }

    test_sram!(test_sram_1, PARAMS_1);
    test_sram!(test_sram_2, PARAMS_2, ignore = "slow");
    test_sram!(test_sram_3, PARAMS_3, ignore = "slow");
    test_sram!(test_sram_4, PARAMS_4, ignore = "slow");
    test_sram!(test_sram_5, PARAMS_5, ignore = "slow");
    test_sram!(test_sram_6, PARAMS_6, ignore = "slow");
    test_sram!(test_sram_7, PARAMS_7, ignore = "slow");
    test_sram!(test_sram_8, PARAMS_8, ignore = "slow");
    test_sram!(test_sram_9, PARAMS_9, ignore = "slow");
    test_sram!(test_sram_10, PARAMS_10, ignore = "slow");
    test_sram!(test_sram_11, PARAMS_11, ignore = "slow");
    test_sram!(test_sram_rocket_1, ROCKET_1, ignore = "slow");
    test_sram!(test_sram_rocket_2, ROCKET_2, ignore = "slow");
    test_sram!(test_sram_rocket_3, ROCKET_3, ignore = "slow");
    test_sram!(test_sram_rocket_4, ROCKET_4, ignore = "slow");
    test_sram!(test_sram_rocket_5, ROCKET_5, ignore = "slow");
    test_sram!(test_sram_rocket_6, ROCKET_6, ignore = "slow");
}
