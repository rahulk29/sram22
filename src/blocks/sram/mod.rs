use self::schematic::fanout_buffer_stage;
use crate::blocks::bitcell_array::replica::ReplicaCellArray;
use crate::blocks::columns::ColumnsPhysicalDesignScript;
use crate::blocks::control::{ControlLogicParams, ControlLogicReplicaV2};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::path::{Path, PathBuf};
use subgeom::bbox::BoundBox;
use subgeom::{Dir, Rect, Span};
use substrate::component::{error, Component};
use substrate::error::ErrorSource;
use substrate::layout::cell::{CellPort, Port, PortId};
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::routing::auto::straps::PlacedStraps;
use substrate::layout::straps::SingleSupplyNet;
use substrate::script::Script;

use super::bitcell_array::replica::ReplicaCellArrayParams;
use super::bitcell_array::SpCellArrayParams;
use super::columns::{ColParams, ColPeripherals, COL_CAPACITANCES, COL_PARAMS};
use super::decoder::{
    Decoder, DecoderParams, DecoderPhysicalDesignParams, DecoderStageParams, DecoderStyle,
    DecoderTree, RoutingStyle, INV_MODEL, INV_PARAMS, NAND2_MODEL, NAND2_PARAMS,
};
use super::gate::{AndParams, GateParams, PrimitiveGateParams};
use super::guard_ring::{GuardRing, GuardRingParams, SupplyRings};
use super::precharge::layout::ReplicaPrechargeParams;
use crate::blocks::columns::layout::DffArray;
use crate::blocks::decoder::DecoderStage;
use crate::blocks::tgatemux::TGateMuxParams;

pub mod layout;
pub mod schematic;
pub mod testbench;
pub mod verilog;

pub const WORDLINE_CAP_PER_CELL: f64 = 0.00000000000001472468276676486 / 12.;
pub const BITLINE_CAP_PER_CELL: f64 = 0.00000000000008859364177937068 / 128.;

#[derive(Debug, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub struct SramConfig {
    pub num_words: usize,
    pub data_width: usize,
    pub mux_ratio: MuxRatio,
    pub write_size: usize,
    #[cfg(feature = "commercial")]
    pub pex_level: Option<calibre::pex::PexLevel>,
}

pub fn parse_sram_config(path: impl AsRef<Path>) -> anyhow::Result<SramConfig> {
    let contents = std::fs::read_to_string(path)?;
    let data = toml::from_str(&contents)?;
    Ok(data)
}

pub struct SramInner {
    params: SramParams,
}

pub struct Sram {
    params: SramParams,
}

pub struct SramPex {
    params: SramPexParams,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum MuxRatio {
    M4 = 4,
    M8 = 8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SramPexParams {
    params: SramParams,
    pex_netlist: PathBuf,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct SramParams {
    wmask_granularity: usize,
    mux_ratio: MuxRatio,
    num_words: usize,
    data_width: usize,
}

impl SramParams {
    pub const fn new(
        wmask_granularity: usize,
        mux_ratio: MuxRatio,
        num_words: usize,
        data_width: usize,
    ) -> Self {
        Self {
            wmask_granularity,
            mux_ratio,
            num_words,
            data_width,
        }
    }

    #[inline]
    pub fn wmask_width(&self) -> usize {
        self.data_width / self.wmask_granularity
    }

    #[inline]
    pub fn row_bits(&self) -> usize {
        (self.num_words / self.mux_ratio as usize).ilog2() as usize
    }

    #[inline]
    pub fn col_select_bits(&self) -> usize {
        (self.mux_ratio as usize).ilog2() as usize
    }

    #[inline]
    pub fn rows(&self) -> usize {
        self.num_words / self.mux_ratio as usize
    }

    #[inline]
    pub fn cols(&self) -> usize {
        self.data_width * self.mux_ratio as usize
    }

    #[inline]
    pub fn wmask_granularity(&self) -> usize {
        self.wmask_granularity
    }

    #[inline]
    pub fn mux_ratio(&self) -> usize {
        self.mux_ratio as usize
    }

    #[inline]
    pub fn num_words(&self) -> usize {
        self.num_words
    }

    #[inline]
    pub fn data_width(&self) -> usize {
        self.data_width
    }

    #[inline]
    pub fn addr_width(&self) -> usize {
        self.num_words.ilog2() as usize
    }

    /// The name of the SRAM cell with these parameters.
    pub fn name(&self) -> arcstr::ArcStr {
        arcstr::format!(
            "sram22_{}x{}m{}w{}",
            self.num_words,
            self.data_width,
            self.mux_ratio as u8,
            self.wmask_granularity()
        )
    }

    pub(crate) fn col_params(&self) -> ColParams {
        let bl_cap = (self.rows() + 4) as f64 * BITLINE_CAP_PER_CELL;
        let pc_scale = f64::max(bl_cap / COL_CAPACITANCES.pc_b / 8.0, 0.4);
        let mux_scale = f64::max(bl_cap / COL_CAPACITANCES.sel / 8.0, 0.5);
        let wrdrvscale = f64::max(bl_cap / COL_CAPACITANCES.we / 8.0, 0.4);
        println!(
            "pc_scale = {pc_scale:.2}, mux_scale = {mux_scale:.2}, wrdrvscale = {wrdrvscale:.2}"
        );
        ColParams {
            pc: COL_PARAMS.pc.scale(pc_scale),
            wrdriver: COL_PARAMS.wrdriver.scale(wrdrvscale),
            mux: TGateMuxParams {
                mux_ratio: self.mux_ratio(),
                ..COL_PARAMS.mux.scale(mux_scale)
            },
            buf: PrimitiveGateParams {
                nwidth: 1_200,
                pwidth: 2_000,
                length: 150,
            },
            cols: self.cols(),
            wmask_granularity: self.wmask_granularity(),
            include_wmask: true,
        }
    }
}

pub struct SramPhysicalDesignScript;

pub struct SramPhysicalDesign {
    bitcells: SpCellArrayParams,
    row_decoder: DecoderParams,
    addr_gate: DecoderStageParams,
    col_decoder: DecoderParams,
    pc_b_buffer: DecoderStageParams,
    wlen_buffer: DecoderStageParams,
    write_driver_en_buffer: DecoderStageParams,
    sense_en_buffer: DecoderStageParams,
    num_dffs: usize,
    rbl_wl_index: usize,
    rbl: ReplicaCellArrayParams,
    replica_pc: ReplicaPrechargeParams,
    col_params: ColParams,
    control: ControlLogicParams,
}

impl Script for SramPhysicalDesignScript {
    type Params = SramParams;
    type Output = SramPhysicalDesign;

    fn run(
        params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let wl_cap = (params.cols() + 4) as f64 * WORDLINE_CAP_PER_CELL * 1.5; // safety factor.
        let col_params = params.col_params();
        let cols = ctx.instantiate_layout::<ColPeripherals>(&col_params)?;
        let rbl_rows = ((params.rows() / 12) + 1) * 2;
        let rbl_wl_index = rbl_rows / 2;
        let rbl = ReplicaCellArrayParams {
            rows: rbl_rows,
            cols: 2,
        };
        let rbl_inst = ctx.instantiate_layout::<ReplicaCellArray>(&rbl)?;

        let addr_gate = DecoderStageParams {
            pd: DecoderPhysicalDesignParams {
                style: DecoderStyle::Minimum,
                dir: Dir::Horiz,
            },
            routing_style: RoutingStyle::Driver,
            max_width: None,
            invs: vec![],
            gate: GateParams::And2(AndParams {
                nand: NAND2_PARAMS,
                inv: INV_PARAMS.scale(4.),
            }),
            num: 2 * params.row_bits(),
            child_sizes: vec![],
        };
        let addr_gate_inst = ctx.instantiate_layout::<DecoderStage>(&addr_gate)?;
        let pc_b_cap =
            COL_CAPACITANCES.pc_b * col_params.cols as f64 * col_params.pc.pull_up_width as f64
                / COL_PARAMS.pc.pull_up_width as f64;
        let wlen_cap = NAND2_MODEL.cin * (params.addr_width() * 2) as f64;
        let wrdrven_cap = COL_CAPACITANCES.we * col_params.wmask_bits() as f64;
        let saen_cap = COL_CAPACITANCES.saen * (col_params.cols / col_params.mux.mux_ratio) as f64;
        let col_sel_cap =
            COL_CAPACITANCES.sel * (col_params.cols / col_params.mux.mux_ratio) as f64;
        let col_sel_b_cap =
            COL_CAPACITANCES.sel_b * (col_params.cols / col_params.mux.mux_ratio) as f64;

        let horiz_buffer = DecoderPhysicalDesignParams {
            style: DecoderStyle::Minimum,
            dir: Dir::Horiz,
        };
        let vert_buffer = DecoderPhysicalDesignParams {
            style: DecoderStyle::Minimum,
            dir: Dir::Vert,
        };

        // Figure out the best width allocation to equalize lengths of the various buffers.
        let mut pc_b_buffer = DecoderStageParams {
            max_width: None,
            ..fanout_buffer_stage(horiz_buffer, pc_b_cap)
        };
        let mut col_decoder = DecoderParams {
            pd: DecoderPhysicalDesignParams {
                style: DecoderStyle::Relaxed,
                dir: Dir::Horiz,
            },
            max_width: None,
            // TODO use tgate mux input cap
            tree: DecoderTree::new(params.col_select_bits(), col_sel_cap + col_sel_b_cap),
        };
        let mut sense_en_buffer = DecoderStageParams {
            max_width: None,
            ..fanout_buffer_stage(horiz_buffer, saen_cap)
        };
        let mut write_driver_en_buffer = DecoderStageParams {
            max_width: None,
            ..fanout_buffer_stage(horiz_buffer, wrdrven_cap)
        };

        // Add inverters to pc_b buffer to equalize wrdrven and pc_b delay.
        let col_dsn = ctx.run_script::<ColumnsPhysicalDesignScript>(&col_params)?;
        let pc_b_delay_invs = ((1.1
            * (f64::max(
                col_dsn.nand.time_constant(col_dsn.cl_max)
                    + write_driver_en_buffer.time_constant(wrdrven_cap),
                sense_en_buffer.time_constant(saen_cap),
            ) - pc_b_buffer.time_constant(pc_b_cap))
            / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)))
            / 2.0)
            .round() as usize
            * 2;
        let mut new_invs = vec![pc_b_buffer.gate.first_gate_sizing(); pc_b_delay_invs];
        new_invs.extend(pc_b_buffer.invs.drain(..));
        pc_b_buffer.invs = new_invs;

        let row_decoder_tree = DecoderTree::new(params.row_bits(), wl_cap);
        let decoder_delay_invs = (f64::max(
            4.0,
            1.1 * row_decoder_tree.root.time_constant(wl_cap)
                / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)),
        ) / 2.0)
            .round() as usize
            * 2
            + 2;
        let write_driver_delay_invs = (f64::max(
            2.0,
            0.25 * row_decoder_tree.root.time_constant(wl_cap)
                / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)),
        ) / 2.0)
            .round() as usize
            * 2
            + 9;
        println!("using {write_driver_delay_invs} inverters for write driver delay chain");
        let control = ControlLogicParams {
            decoder_delay_invs,
            write_driver_delay_invs,
        };

        let control_inst = ctx.instantiate_layout::<ControlLogicReplicaV2>(&control)?;

        let col_dec_inst = ctx.instantiate_layout::<Decoder>(&col_decoder)?;
        let pc_b_buffer_inst = ctx.instantiate_layout::<DecoderStage>(&pc_b_buffer)?;
        let sense_en_buffer_inst = ctx.instantiate_layout::<DecoderStage>(&sense_en_buffer)?;
        let write_driver_en_buffer_inst =
            ctx.instantiate_layout::<DecoderStage>(&write_driver_en_buffer)?;
        let col_dec_wh = col_dec_inst.brect().width() * col_dec_inst.brect().height();
        let pc_b_buffer_wh = pc_b_buffer_inst.brect().width() * pc_b_buffer_inst.brect().height();
        let sense_en_buffer_wh =
            sense_en_buffer_inst.brect().width() * sense_en_buffer_inst.brect().height();
        let write_driver_en_buffer_wh = write_driver_en_buffer_inst.brect().width()
            * write_driver_en_buffer_inst.brect().height();
        let mut total_wh =
            col_dec_wh + pc_b_buffer_wh + sense_en_buffer_wh + write_driver_en_buffer_wh;
        let num_dffs = params.addr_width() + 2;
        let dffs_inst = ctx.instantiate_layout::<DffArray>(&num_dffs)?;

        let mut available_height = [
            cols.brect().height()
            - dffs_inst.brect().height()
            - 3_500 // DFF offset
            - 1_400 * params.addr_width() as i64,
            rbl_inst.brect().height(),
            control_inst.brect().width(),
        ]
        .into_iter()
        .max()
        .unwrap()
            - 3 * 6_000; // Offset between buffers

        let col_dec_max_width = std::cmp::max(
            available_height * col_dec_wh / total_wh,
            col_dec_inst.brect().width(),
        );
        available_height -= col_dec_max_width;
        total_wh -= col_dec_wh;
        let pc_b_buffer_max_width = available_height * pc_b_buffer_wh / total_wh;
        let sense_en_buffer_max_width = available_height * sense_en_buffer_wh / total_wh;
        let write_driver_en_buffer_max_width =
            available_height * write_driver_en_buffer_wh / total_wh;
        col_decoder.max_width = Some(col_dec_max_width);
        pc_b_buffer.max_width = Some(pc_b_buffer_max_width);
        sense_en_buffer.max_width = Some(sense_en_buffer_max_width);
        write_driver_en_buffer.max_width = Some(write_driver_en_buffer_max_width);

        let wlen_buffer = DecoderStageParams {
            max_width: Some(addr_gate_inst.brect().height() - 2_000),
            ..fanout_buffer_stage(vert_buffer, wlen_cap)
        };
        println!("wlen_buffer: {:?}", wlen_buffer);

        assert_eq!(decoder_delay_invs % 2, 0);
        Ok(Self::Output {
            bitcells: SpCellArrayParams {
                rows: params.rows(),
                cols: params.cols(),
                mux_ratio: params.mux_ratio(),
            },
            row_decoder: DecoderParams {
                pd: DecoderPhysicalDesignParams {
                    style: DecoderStyle::RowMatched,
                    dir: Dir::Horiz,
                },
                max_width: None,
                tree: row_decoder_tree,
            },
            addr_gate,
            // TODO: change decoder tree to provide correct fanout for inverted output
            col_decoder,
            pc_b_buffer,
            wlen_buffer,
            write_driver_en_buffer,
            sense_en_buffer,
            num_dffs,
            rbl_wl_index,
            rbl: ReplicaCellArrayParams {
                rows: rbl_rows,
                cols: 2,
            },
            replica_pc: ReplicaPrechargeParams {
                cols: 2,
                inner: col_params.pc,
            },
            col_params,
            control: ControlLogicParams {
                decoder_delay_invs,
                write_driver_delay_invs,
            },
        })
    }
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

        let m0 = ctx.layers().get(Selector::Metal(0))?;
        let m1 = ctx.layers().get(Selector::Metal(1))?;
        let m2 = ctx.layers().get(Selector::Metal(2))?;
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

        for (layer, dir) in [(m1, Dir::Vert), (m2, Dir::Horiz)] {
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

                let mut targets = Vec::new();
                if strap.upper_boundary {
                    targets.push(ring.dir_rects(!dir)[1]);
                }
                if strap.lower_boundary {
                    targets.push(ring.dir_rects(!dir)[0]);
                }
                for target in targets {
                    let (below_rect, above_rect) = if layer == m2 {
                        (target, r)
                    } else {
                        (r, target)
                    };
                    let viap = ViaParams::builder()
                        .layers(m1, m2)
                        .geometry(below_rect, above_rect)
                        .expand(ViaExpansion::LongerDirection)
                        .build();
                    ctx.instantiate::<Via>(&viap)?.add_to(ctx)?;
                }
                if layer == m1 {
                    let mut targets = Vec::new();
                    if strap.upper_boundary {
                        targets.push(ring.inner_hrects()[1]);
                    }
                    if strap.lower_boundary {
                        targets.push(ring.inner_hrects()[0]);
                    }
                    for target in targets {
                        let viap = ViaParams::builder()
                            .layers(m0, m1)
                            .geometry(target, r)
                            .expand(ViaExpansion::LongerDirection)
                            .build();
                        ctx.instantiate::<Via>(&viap)?.add_to(ctx)?;
                    }
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
        for (pin, width) in [
            ("dout", self.params.data_width()),
            ("din", self.params.data_width()),
            ("wmask", self.params.wmask_width()),
            ("addr", self.params.addr_width()),
            ("we", 1),
            ("ce", 1),
            ("clk", 1),
            ("reset_b", 1),
        ] {
            for i in 0..width {
                let port_id = PortId::new(pin, i);
                let rect = sram.port(port_id.clone())?.largest_rect(m1)?;
                let rect = rect.with_vspan(
                    rect.vspan()
                        .add_point(ctx.bbox().into_rect().side(subgeom::Side::Bot)),
                );
                ctx.draw_rect(m1, rect);
                ctx.add_port(CellPort::builder().id(port_id).add(m1, rect).build())?;
            }
        }

        Ok(())
    }
}

impl Component for SramPex {
    type Params = SramPexParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("{}_pex", self.params.params.name())
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        use std::fmt::Write;

        let inner = ctx.instantiate::<Sram>(&self.params.params)?.named("Xdut");
        let mut s = inner.name().to_string();
        for port in inner.ports()? {
            ctx.bus_port(port.name(), port.width(), port.direction());
            for i in 0..port.width() {
                if port.width > 1 {
                    write!(&mut s, " {}[{}]", port.name(), i).unwrap();
                } else {
                    write!(&mut s, " {}", port.name()).unwrap();
                }
            }
        }
        write!(&mut s, " {}", inner.module().local().unwrap().name()).unwrap();
        ctx.set_spice(s);
        Ok(())
    }

    fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Err(ErrorSource::Component(error::Error::ViewUnsupported(
            substrate::component::View::Layout,
        ))
        .into())
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use self::testbench::TestSequence;
    use self::verilog::save_1rw_verilog;
    use crate::blocks::sram::testbench::verify::verify_simulation;
    use crate::paths::*;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use substrate::schematic::netlist::NetlistPurpose;

    use super::*;

    pub(crate) const SRAM22_64X4M4W2: SramParams = SramParams::new(2, MuxRatio::M4, 64, 4);

    pub(crate) const SRAM22_64X24M4W24: SramParams = SramParams::new(24, MuxRatio::M4, 64, 24);

    pub(crate) const SRAM22_64X32M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 64, 32);

    pub(crate) const SRAM22_64X32M4W32: SramParams = SramParams::new(32, MuxRatio::M4, 64, 32);

    pub(crate) const SRAM22_256X32M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 256, 32);

    pub(crate) const SRAM22_512X32M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 512, 32);

    pub(crate) const SRAM22_512X32M4W32: SramParams = SramParams::new(32, MuxRatio::M4, 512, 32);

    pub(crate) const SRAM22_512X64M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 512, 64);

    pub(crate) const SRAM22_1024X32M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 1024, 32);

    pub(crate) const SRAM22_1024X32M8W32: SramParams = SramParams::new(32, MuxRatio::M8, 1024, 32);

    pub(crate) const SRAM22_1024X64M8W32: SramParams = SramParams::new(32, MuxRatio::M8, 1024, 64);

    pub(crate) const SRAM22_2048X32M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 2048, 32);

    pub(crate) const SRAM22_2048X64M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 2048, 64);

    pub(crate) const SRAM22_4096X8M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 4096, 8);

    pub(crate) const SRAM22_4096X32M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 4096, 32);

    pub(crate) const SRAM22_4096X128M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 4096, 128);

    pub(crate) const SRAM22_2048X256M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 2048, 256);

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

                let verilog_path = out_verilog(&work_dir, &*$params.name());
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

                    let pex_path = out_spice(&work_dir, "pex_schematic");
                    let pex_dir = work_dir.join("pex");
                    let pex_level = calibre::pex::PexLevel::Rc;
                    let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
                    ctx.write_schematic_to_file_for_purpose::<Sram>(
                        &$params,
                        &pex_path,
                        NetlistPurpose::Pex,
                    ).expect("failed to write pex source netlist");
                    let mut opts = std::collections::HashMap::with_capacity(1);
                    opts.insert("level".into(), pex_level.as_str().into());

                    ctx.run_pex(substrate::verification::pex::PexInput {
                        work_dir: pex_dir,
                        layout_path: gds_path.clone(),
                        layout_cell_name: $params.name().clone(),
                        layout_format: substrate::layout::LayoutFormat::Gds,
                        source_paths: vec![pex_path],
                        source_cell_name: $params.name().clone(),
                        pex_netlist_path: pex_netlist_path.clone(),
                        ground_net: "vss".to_string(),
                        opts,
                    }).expect("failed to run pex");

                    let seq = TestSequence::Short;
                    let corners = ctx.corner_db();
                    let mut handles = Vec::new();
                    for vdd in [1.8] {
                        for corner in corners.corners() {
                            let corner = corner.clone();
                            let params = $params.clone();
                            let pex_netlist = Some((pex_netlist_path.clone(), pex_level));
                            let work_dir = work_dir.clone();
                            handles.push(std::thread::spawn(move || {
                                let ctx = setup_ctx();
                                let tb = crate::blocks::sram::testbench::tb_params(params, vdd, seq, pex_netlist);
                                let work_dir = work_dir.join(format!(
                                    "{}_{:.2}_{}",
                                    corner.name(),
                                    vdd,
                                    seq.as_str(),
                                ));
                                let data = ctx.write_simulation_with_corner::<crate::blocks::sram::testbench::SramTestbench>(
                                    &tb,
                                    &work_dir,
                                    corner.clone(),
                                )
                                .expect("failed to run simulation");
                                verify_simulation(&work_dir, &data, &tb).map_err(|e| panic!("failed to verify simulation in corner {} with vdd={vdd:.2}, seq={seq}: {e:#?}", corner.name())).unwrap();
                                println!(
                                    "Simulated corner {} with Vdd = {}, seq = {}",
                                    corner.name(),
                                    vdd,
                                    seq,
                                );
                            }));
                        }
                    }
                    for handle in handles {
                        handle.join().expect("failed to join thread");
                    }

                    // crate::abs::run_abstract(
                    //     &work_dir,
                    //     &$params.name(),
                    //     crate::paths::out_lef(&work_dir, "abstract"),
                    //     &gds_path,
                    //     &verilog_path,
                    // )
                    // .expect("failed to write abstract");
                    // println!("{}: done writing abstract", stringify!($name));

                    // let timing_spice_path = out_spice(&work_dir, "timing_schematic");
                    // ctx.write_schematic_to_file_for_purpose::<Sram>(
                    //     &TINY_SRAM,
                    //     &timing_spice_path,
                    //     NetlistPurpose::Timing,
                    // )
                    // .expect("failed to write timing schematic");

                    // let params = liberate_mx::LibParams::builder()
                    //     .work_dir(work_dir.join("lib"))
                    //     .output_file(crate::paths::out_lib(&work_dir, "timing_tt_025C_1v80.schematic"))
                    //     .corner("tt")
                    //     .cell_name(&*$params.name())
                    //     .num_words($params.num_words)
                    //     .data_width($params.data_width)
                    //     .addr_width($params.addr_width)
                    //     .wmask_width($params.wmask_width)
                    //     .mux_ratio($params.mux_ratio)
                    //     .has_wmask(true)
                    //     .source_paths(vec![timing_spice_path])
                    //     .build()
                    //     .unwrap();
                    // crate::liberate::generate_sram_lib(&params).expect("failed to write lib");
                }
            }
        };
    }

    test_sram!(test_sram22_64x4m4w2, SRAM22_64X4M4W2);
    test_sram!(test_sram22_64x24m4w24, SRAM22_64X24M4W24, ignore = "slow");
    test_sram!(test_sram22_64x32m4w8, SRAM22_64X32M4W8, ignore = "slow");
    test_sram!(test_sram22_64x32m4w32, SRAM22_64X32M4W32, ignore = "slow");
    test_sram!(test_sram22_256x32m4w8, SRAM22_256X32M4W8, ignore = "slow");
    test_sram!(test_sram22_512x32m4w8, SRAM22_512X32M4W8, ignore = "slow");
    test_sram!(test_sram22_512x32m4w32, SRAM22_512X32M4W32, ignore = "slow");
    test_sram!(test_sram22_512x64m4w8, SRAM22_512X64M4W8, ignore = "slow");
    test_sram!(test_sram22_1024x32m8w8, SRAM22_1024X32M8W8, ignore = "slow");
    test_sram!(
        test_sram22_1024x32m8w32,
        SRAM22_1024X32M8W32,
        ignore = "slow"
    );
    test_sram!(
        test_sram22_1024x64m8w32,
        SRAM22_1024X64M8W32,
        ignore = "slow"
    );
    test_sram!(test_sram22_2048x32m8w8, SRAM22_2048X32M8W8, ignore = "slow");
    test_sram!(test_sram22_2048x64m4w8, SRAM22_2048X64M4W8, ignore = "slow");
    test_sram!(test_sram22_4096x8m8w8, SRAM22_4096X8M8W8, ignore = "slow");
    test_sram!(test_sram22_4096x32m8w8, SRAM22_4096X32M8W8, ignore = "slow");
    test_sram!(
        test_sram22_4096x128m8w8,
        SRAM22_4096X128M8W8,
        ignore = "slow"
    );
    test_sram!(
        test_sram22_2048x256m4w8,
        SRAM22_2048X256M4W8,
        ignore = "slow"
    );
}
