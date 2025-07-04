use self::schematic::fanout_buffer_stage;
use crate::blocks::bitcell_array::replica::ReplicaCellArray;
use crate::blocks::columns::ColumnsPhysicalDesignScript;
use crate::blocks::control::{ControlLogicParams, ControlLogicReplicaV2};
use crate::blocks::precharge::layout::ReplicaPrecharge;
use crate::blocks::precharge::PrechargeParams;
use arcstr::ArcStr;
use layout::{ReplicaColumnMos, ReplicaColumnMosParams, ReplicaMetalRoutingParams};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::path::{Path, PathBuf};
use subgeom::bbox::BoundBox;
use subgeom::{snap_to_grid, Corner, Dir, Point, Rect, Span};
use substrate::component::{error, Component};
use substrate::data::SubstrateCtx;
use substrate::error::ErrorSource;
use substrate::layout::cell::{CellPort, Element, Port, PortConflictStrategy, PortId};
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::group::Group;
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerSpec;
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::routing::auto::straps::PlacedStraps;
use substrate::layout::straps::SingleSupplyNet;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::script::Script;

use super::bitcell_array::replica::ReplicaCellArrayParams;
use super::bitcell_array::SpCellArrayParams;
use super::columns::{self, ColParams, ColPeripherals, COL_CAPACITANCES, COL_PARAMS};
use super::decoder::{
    Decoder, DecoderParams, DecoderPhysicalDesignParams, DecoderStageParams, DecoderStyle,
    DecoderTree, RoutingStyle, INV_MODEL, INV_PARAMS, NAND2_MODEL, NAND2_PARAMS,
};
use super::gate::{AndParams, GateParams};
use super::guard_ring::{GuardRing, GuardRingParams, SupplyRings};
use super::precharge::layout::ReplicaPrechargeParams;
use crate::blocks::columns::layout::DffArray;
use crate::blocks::decoder::{DecoderStage, NAND3_MODEL};
use crate::blocks::tgatemux::{TGateMux, TGateMuxParams};

pub mod layout;
pub mod schematic;
pub mod testbench;

pub const WORDLINE_CAP_PER_CELL: f64 = 0.00000000000001472468276676486 / 12.;
pub const BITLINE_CAP_PER_CELL: f64 = 0.00000000000008859364177937068 / 128.;

/// The threshold at which further decoder scaling does not help,
/// since delay is dominated by routing resistance/capacitance.
pub const WORDLINE_CAP_MAX: f64 = 500e-15;

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

pub struct SramAggregator {
    params: Vec<SramParams>,
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
        let wrdrvscale = f64::max(bl_cap / COL_CAPACITANCES.we / 6.0, 0.4);
        ColParams {
            pc: COL_PARAMS.pc.scale(pc_scale),
            wrdriver: COL_PARAMS.wrdriver.scale(wrdrvscale),
            mux: TGateMuxParams {
                mux_ratio: self.mux_ratio(),
                ..COL_PARAMS.mux.scale(mux_scale)
            },
            latch: COL_PARAMS.latch,
            cols: self.cols(),
            wmask_granularity: self.wmask_granularity(),
            include_wmask: true,
        }
    }
}

pub struct SramPhysicalDesignScript;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SramPhysicalDesign {
    pub(crate) bitcells: SpCellArrayParams,
    pub(crate) row_decoder: DecoderParams,
    pub(crate) addr_gate: DecoderStageParams,
    pub(crate) col_decoder: DecoderParams,
    pub(crate) pc_b_buffer: DecoderStageParams,
    pub(crate) wlen_buffer: DecoderStageParams,
    pub(crate) write_driver_en_buffer: DecoderStageParams,
    pub(crate) sense_en_buffer: DecoderStageParams,
    pub(crate) num_dffs: usize,
    pub(crate) rbl_wl_index: usize,
    pub(crate) rbl: ReplicaCellArrayParams,
    pub(crate) replica_pc: ReplicaPrechargeParams,
    pub(crate) replica_nmos: ReplicaColumnMosParams,
    pub(crate) replica_routing: ReplicaMetalRoutingParams,
    pub(crate) col_params: ColParams,
    pub(crate) control: ControlLogicParams,
    pub(crate) pc_b_routing_tracks: i64,
    pub(crate) sense_en_routing_tracks: i64,
    pub(crate) write_driver_en_routing_tracks: i64,
    pub(crate) col_dec_routing_tracks: i64,
}

impl Script for SramPhysicalDesignScript {
    type Params = SramParams;
    type Output = SramPhysicalDesign;

    fn run(
        params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let wl_cap = (params.cols() + 4) as f64 * WORDLINE_CAP_PER_CELL * 1.5; // safety factor.
        let clamped_wl_cap = f64::min(wl_cap, WORDLINE_CAP_MAX);
        let mut col_params = params.col_params();
        let cols = ctx.instantiate_layout::<ColPeripherals>(&col_params)?;
        // +2 for dummy bitcells, then div_ceil by 6 and multiply by 2 for at least 0.9/3 = 0.3 V
        // differential and even number of rows.
        let rbl_ratio = 6;
        let rbl_rows = (params.rows() + 2).div_ceil(4 * rbl_ratio) * 4;
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
            use_multi_finger_invs: true,
            dont_connect_outputs: false,
            child_sizes: vec![],
        };
        let addr_gate_inst = ctx.instantiate_layout::<DecoderStage>(&addr_gate)?;
        let pc_b_cap = COL_CAPACITANCES.pc_b
            * (col_params.cols + 4) as f64
            * col_params.pc.pull_up_width as f64
            / COL_PARAMS.pc.pull_up_width as f64;
        let wlen_cap = NAND2_MODEL.cin * (params.addr_width() * 2) as f64;
        let wrdrven_cap = COL_CAPACITANCES.we * col_params.wmask_bits() as f64;
        let saen_cap = COL_CAPACITANCES.saen * (col_params.cols / col_params.mux.mux_ratio) as f64;
        let col_sel_cap = COL_CAPACITANCES.sel
            * (col_params.cols / col_params.mux.mux_ratio) as f64
            * col_params.mux.pwidth as f64
            / COL_PARAMS.mux.pwidth as f64;
        let col_sel_b_cap = COL_CAPACITANCES.sel_b
            * (col_params.cols / col_params.mux.mux_ratio) as f64
            * col_params.mux.pwidth as f64
            / COL_PARAMS.mux.pwidth as f64;

        let horiz_buffer = DecoderPhysicalDesignParams {
            style: DecoderStyle::Minimum,
            dir: Dir::Horiz,
        };
        let vert_buffer = DecoderPhysicalDesignParams {
            style: DecoderStyle::Minimum,
            dir: Dir::Vert,
        };

        let wlen_buffer = DecoderStageParams {
            max_width: Some(addr_gate_inst.brect().height() - 2_000),
            ..fanout_buffer_stage(vert_buffer, wlen_cap)
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
            use_multi_finger_invs: true,
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
        let pcb_tau = pc_b_buffer.time_constant(pc_b_cap);
        let wrdrven_tau = write_driver_en_buffer.time_constant(wrdrven_cap)
            + col_dsn.nand.time_constant(col_dsn.cl_max);
        let sae_tau = sense_en_buffer.time_constant(saen_cap);
        let pc_b_delay_invs = ((1.2 * (1.35 * f64::max(wrdrven_tau, sae_tau) - pcb_tau)
            / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)))
            / 2.0)
            .max(0.)
            .ceil() as usize
            * 2
            + 8;
        let wrdrven_set_delay_invs = (((1.1 * pcb_tau - wrdrven_tau)
            / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)))
            / 2.0)
            .max(1.)
            .round() as usize
            * 2;
        let row_decoder_tree = DecoderTree::new(params.row_bits(), clamped_wl_cap);
        let decoder_delay_invs = (f64::max(
            4.0,
            (row_decoder_tree.root.time_constant(wl_cap)
                + addr_gate.time_constant(NAND3_MODEL.cin * 4.)
                + wlen_buffer.time_constant(wlen_cap)
                - f64::min(sae_tau, wrdrven_tau))
                / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)),
        ) / 2.0)
            .round() as usize
            * 2
            + 2;
        let wlen_pulse_invs = (f64::max(
            2.0,
            (0.25 * row_decoder_tree.root.time_constant(wl_cap)
                + 6.0
                    * (row_decoder_tree.root.time_constant(wl_cap)
                        - row_decoder_tree.root.time_constant(clamped_wl_cap)))
                / (INV_MODEL.res * (INV_MODEL.cin + INV_MODEL.cout)),
        ) / 2.0)
            .round() as usize
            * 2
            + 9;
        let control = ControlLogicParams {
            decoder_delay_invs,
            wlen_pulse_invs,
            pc_set_delay_invs: pc_b_delay_invs,
            wrdrven_set_delay_invs,
            wrdrven_rst_delay_invs: 0, // TODO: Implement delay to equalize sense amp and
                                       // write driver rest delay
        };
        let row_decoder = DecoderParams {
            pd: DecoderPhysicalDesignParams {
                style: DecoderStyle::RowMatched,
                dir: Dir::Horiz,
            },
            max_width: None,
            tree: row_decoder_tree,
            use_multi_finger_invs: true,
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

        // Subtract DFF offset and routing tracks.
        let mut available_height = [
            cols.brect().height()
                - dffs_inst.brect().height()
                - 5_500
                - 1_400 * params.addr_width() as i64,
            rbl_inst.brect().height(),
            control_inst.brect().width(),
        ]
        .into_iter()
        .max()
        .unwrap()
            - 4 * 6_000; // Offset between buffers
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

        let col_dec_inst = ctx.instantiate_layout::<Decoder>(&col_decoder)?;
        let row_dec_inst = ctx.instantiate_layout::<Decoder>(&row_decoder)?;
        let col_dec_wh = col_dec_inst.brect().width() * col_dec_inst.brect().height();
        let col_dec_width_to_match_row_dec = col_dec_wh / row_dec_inst.brect().height();
        let col_dec_max_width = if col_dec_width_to_match_row_dec < 2 * col_dec_max_width {
            std::cmp::max(col_dec_width_to_match_row_dec, col_dec_max_width)
        } else {
            col_dec_max_width
        };
        col_decoder.max_width = Some(col_dec_max_width);
        pc_b_buffer.max_width = Some(std::cmp::max(pc_b_buffer_max_width, 6_000));
        sense_en_buffer.max_width = Some(std::cmp::max(sense_en_buffer_max_width, 6_000));
        write_driver_en_buffer.max_width =
            Some(std::cmp::max(write_driver_en_buffer_max_width, 6_000));

        assert_eq!(decoder_delay_invs % 2, 0);
        let pc_b_routing_tracks =
            ((pc_b_cap / (COL_CAPACITANCES.pc_b * 256.)).ceil() as i64).clamp(2, 8);
        let write_driver_en_routing_tracks =
            ((wrdrven_cap / (COL_CAPACITANCES.we * 8.)).ceil() as i64).clamp(2, 8);
        let sense_en_routing_tracks =
            ((saen_cap / (COL_CAPACITANCES.saen * 32.)).ceil() as i64).clamp(2, 8);
        let col_dec_routing_tracks =
            ((col_sel_cap / (COL_CAPACITANCES.sel * 64.)).ceil() as i64).clamp(2, 4);

        col_params.mux.sel_width = 320 + (320 + 360) * (col_dec_routing_tracks - 1);
        col_params.pc.en_b_width = 320 + (320 + 360) * (pc_b_routing_tracks - 1);

        let mux_inst = ctx.instantiate_layout::<TGateMux>(&col_params.mux)?;
        let replica_pc = ReplicaPrechargeParams {
            cols: 2,
            inner: PrechargeParams {
                en_b_width: 360,
                ..col_params.pc.scale(1. / rbl_ratio as f64)
            },
        };
        let replica_pc_inst = ctx.instantiate_layout::<ReplicaPrecharge>(&replica_pc)?;
        let replica_nmos = ReplicaColumnMosParams {
            max_height: replica_pc_inst.brect().height(),
            gate_width_n: snap_to_grid(3_360usize.div_ceil(rbl_ratio) as i64, 50),
            drain_width_n: snap_to_grid(
                ((col_params.mux.nwidth * (params.mux_ratio() as i64 + 1)
                    + col_params.wrdriver.nwidth_driver) as usize)
                    .div_ceil(rbl_ratio) as i64,
                50,
            ),
            drain_width_p: snap_to_grid(
                ((col_params.mux.pwidth * (params.mux_ratio() as i64 + 1)
                    + col_params.wrdriver.pwidth_driver) as usize)
                    .div_ceil(rbl_ratio) as i64,
                50,
            ),
            length: 150,
        };
        let replica_nmos_inst = ctx.instantiate_layout::<ReplicaColumnMos>(&replica_nmos)?;

        Ok(Self::Output {
            bitcells: SpCellArrayParams {
                rows: params.rows(),
                cols: params.cols(),
                mux_ratio: params.mux_ratio(),
            },
            row_decoder,
            addr_gate,
            // TODO: change decoder tree to provide correct fanout for inverted output
            col_decoder,
            pc_b_buffer,
            wlen_buffer,
            write_driver_en_buffer,
            sense_en_buffer,
            num_dffs,
            rbl_wl_index,
            rbl,
            replica_pc,
            replica_nmos,
            // TODO: Fix m1_area replica calculation (currently conservative).
            replica_routing: ReplicaMetalRoutingParams {
                m0_area: ((col_params.wrdriver.pwidth_driver + col_params.wrdriver.nwidth_driver)
                    as usize)
                    .div_ceil(rbl_ratio) as i64
                    * 1_080,
                m1_area: (mux_inst.brect().height() as usize * params.mux_ratio())
                    .div_ceil(rbl_ratio) as i64
                    * 1_080,
                max_height: std::cmp::max(
                    replica_pc_inst.brect().height(),
                    replica_nmos_inst.brect().height(),
                ),
            },
            col_params,
            control,
            pc_b_routing_tracks,
            write_driver_en_routing_tracks,
            sense_en_routing_tracks,
            col_dec_routing_tracks,
        })
    }
}

impl Component for SramInner {
    type Params = SramParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self { params: *params })
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
        Ok(Self { params: *params })
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

    // Draws guard ring and shifts coordinates such that origin is at lower left corner.
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let mut group = Group::new();
        let sram = ctx.instantiate::<SramInner>(&self.params)?;
        ctx.set_metadata(*sram.cell().get_metadata::<columns::layout::Metadata>());
        let brect = sram.brect();

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
                let strap_rect = strap.rect;
                let ring = match strap.net {
                    SingleSupplyNet::Vss => rings.vss,
                    SingleSupplyNet::Vdd => rings.vdd,
                };
                assert_ne!(strap_rect.area(), 0);
                let lower = if strap.lower_boundary {
                    ring.outer().span(dir).start()
                } else {
                    strap_rect.span(dir).start()
                };
                let upper = if strap.upper_boundary {
                    ring.outer().span(dir).stop()
                } else {
                    strap_rect.span(dir).stop()
                };

                let r = Rect::span_builder()
                    .with(dir, Span::new(lower, upper))
                    .with(!dir, strap_rect.span(!dir))
                    .build();
                if layer == m2 {
                    group.add_port_with_strategy(
                        CellPort::with_shape(
                            match strap.net {
                                SingleSupplyNet::Vdd => "vdd",
                                SingleSupplyNet::Vss => "vss",
                            },
                            m2,
                            r,
                        ),
                        PortConflictStrategy::Merge,
                    )?;
                }

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
                    group.add_instance(ctx.instantiate::<Via>(&viap)?);
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
                        group.add_instance(ctx.instantiate::<Via>(&viap)?);
                    }
                }
                group.add(Element::new(LayerSpec::drawing(layer), r));
            }
        }

        for port in ["vdd", "vss"] {
            group.add_port_with_strategy(
                ring.port(format!("ring_{port}"))?
                    .into_cell_port()
                    .named(port),
                PortConflictStrategy::Merge,
            )?;
        }

        group.add_port_with_strategy(sram.port("vdd")?, PortConflictStrategy::Merge)?;
        group.add_port_with_strategy(sram.port("vss")?, PortConflictStrategy::Merge)?;
        // Route pins to edge of guard ring
        for (pin, width) in [
            ("dout", self.params.data_width()),
            ("din", self.params.data_width()),
            ("wmask", self.params.wmask_width()),
            ("addr", self.params.addr_width()),
            ("we", 1),
            ("ce", 1),
            ("clk", 1),
            ("rstb", 1),
        ] {
            for i in 0..width {
                let port_id = PortId::new(pin, i);
                let rect = sram.port(port_id.clone())?.largest_rect(m1)?;
                let rect = rect.with_vspan(
                    rect.vspan()
                        .add_point(ring.bbox().into_rect().side(subgeom::Side::Bot)),
                );
                group.add(Element::new(LayerSpec::drawing(m1), rect));
                group.add_port(CellPort::builder().id(port_id).add(m1, rect).build())?;
            }
        }

        group.add_instance(sram);
        group.add_instance(ring);

        group.place(Corner::LowerLeft, Point::zero());
        ctx.add_ports(group.ports())?;
        ctx.draw(group)?;

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

impl Component for SramAggregator {
    type Params = Vec<SramParams>;

    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sram22_sram_aggregator")
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let [clk, rstb] = ctx.ports(["clk", "rstb"], Direction::Input);
        for (i, sram) in self.params.iter().enumerate() {
            let we = ctx.port(format!("we_{i}"), Direction::Input);
            let ce = ctx.port(format!("ce_{i}"), Direction::Input);
            let addr = ctx.bus_port(format!("addr_{i}"), sram.addr_width(), Direction::Input);
            let wmask = ctx.bus_port(format!("wmask_{i}"), sram.wmask_width(), Direction::Input);
            let din = ctx.bus_port(format!("din_{i}"), sram.data_width(), Direction::Input);
            let dout = ctx.bus_port(format!("dout_{i}"), sram.data_width(), Direction::Output);
            ctx.instantiate::<Sram>(sram)?
                .with_connections([
                    ("vdd", vdd),
                    ("vss", vss),
                    ("clk", clk),
                    ("we", we),
                    ("ce", ce),
                    ("rstb", rstb),
                    ("addr", addr),
                    ("wmask", wmask),
                    ("din", din),
                    ("dout", dout),
                ])
                .add_to(ctx);
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use crate::paths::*;
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use crate::verilog::save_1rw_verilog;
    use layout::{ReplicaColumnMos, ReplicaColumnMosParams};

    use super::*;

    pub(crate) const SRAM22_64X24M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 64, 24);

    pub(crate) const SRAM22_64X32M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 64, 32);

    pub(crate) const SRAM22_128X16M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 128, 16);

    pub(crate) const SRAM22_128X24M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 128, 24);

    pub(crate) const SRAM22_128X32M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 128, 32);

    pub(crate) const SRAM22_256X8M8W1: SramParams = SramParams::new(1, MuxRatio::M8, 256, 8);

    pub(crate) const SRAM22_256X16M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 256, 16);

    pub(crate) const SRAM22_256X32M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 256, 32);

    pub(crate) const SRAM22_256X64M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 256, 64);

    pub(crate) const SRAM22_256X128M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 256, 128);

    pub(crate) const SRAM22_512X8M8W1: SramParams = SramParams::new(1, MuxRatio::M8, 512, 8);

    pub(crate) const SRAM22_512X32M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 512, 32);

    pub(crate) const SRAM22_512X64M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 512, 64);

    pub(crate) const SRAM22_512X128M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 512, 128);

    pub(crate) const SRAM22_1024X8M8W1: SramParams = SramParams::new(1, MuxRatio::M8, 1024, 8);

    pub(crate) const SRAM22_1024X32M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 1024, 32);

    pub(crate) const SRAM22_1024X64M4W8: SramParams = SramParams::new(8, MuxRatio::M4, 1024, 64);

    pub(crate) const SRAM22_2048X8M8W1: SramParams = SramParams::new(1, MuxRatio::M8, 2048, 8);

    pub(crate) const SRAM22_2048X32M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 2048, 32);

    pub(crate) const SRAM22_4096X8M8W1: SramParams = SramParams::new(1, MuxRatio::M8, 4096, 8);

    pub(crate) const SRAM22_4096X32M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 4096, 32);

    pub(crate) const SRAM22_8192X32M8W8: SramParams = SramParams::new(8, MuxRatio::M8, 8192, 32);

    #[test]
    fn test_replica_column_nmos() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_replica_column_nmos");
        ctx.write_layout::<ReplicaColumnMos>(
            &ReplicaColumnMosParams {
                max_height: 2_400,
                gate_width_n: 2_000,
                drain_width_n: 2_000,
                drain_width_p: 2_000,
                length: 150,
            },
            out_gds(work_dir, "layout"),
        )
        .expect("failed to write layout");
    }

    #[test]
    #[ignore = "slow"]
    fn test_sram_aggregator() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sram_aggregator");
        let params = vec![
            SRAM22_64X24M4W8,
            SRAM22_64X32M4W8,
            SRAM22_128X16M4W8,
            SRAM22_128X24M4W8,
            SRAM22_128X32M4W8,
            SRAM22_256X8M8W1,
            SRAM22_256X16M8W8,
            SRAM22_256X32M4W8,
            SRAM22_256X64M4W8,
            SRAM22_256X128M4W8,
            SRAM22_512X8M8W1,
            SRAM22_512X32M4W8,
            SRAM22_512X64M4W8,
            SRAM22_512X128M4W8,
            SRAM22_1024X8M8W1,
            SRAM22_1024X32M8W8,
            SRAM22_1024X64M4W8,
            SRAM22_2048X8M8W1,
            SRAM22_2048X32M8W8,
            SRAM22_4096X8M8W1,
            SRAM22_4096X32M8W8,
            SRAM22_8192X32M8W8,
        ];
        let spice_path = out_spice(&work_dir, "sram22_sram_aggregator");
        ctx.write_schematic_to_file::<SramAggregator>(&params, &spice_path)
            .expect("failed to write schematic");
    }

    macro_rules! test_sram {
        ($name: ident, $params: ident $(, $attr: meta)*) => {
            #[test]
            $(#[$attr])*
            fn $name() {
                let ctx = setup_ctx();
                let work_dir = test_work_dir(stringify!($name));

                let spice_path = out_spice(&work_dir, &*$params.name());
                ctx.write_schematic_to_file::<Sram>(&$params, &spice_path)
                    .expect("failed to write schematic");
                    println!("{}: done writing schematic", stringify!($name));

                let gds_path = out_gds(&work_dir, &*$params.name());
                ctx.write_layout::<Sram>(&$params, &gds_path)
                    .expect("failed to write layout");
                    println!("{}: done writing layout", stringify!($name));

                let verilog_path = out_verilog(&work_dir, &*$params.name());
                save_1rw_verilog(&verilog_path, &$params)
                    .expect("failed to write behavioral model");
                    println!("{}: done writing Verilog model", stringify!($name));

                #[cfg(feature = "commercial")]
                {
                    use self::testbench::TestSequence;
                    use crate::blocks::sram::testbench::verify::verify_simulation;
                    use rust_decimal::Decimal;
                    use rust_decimal_macros::dec;
                    use substrate::schematic::netlist::NetlistPurpose;
                    use calibre::drc::{run_drc, DrcParams};
                    use calibre::lvs::{run_lvs, LvsParams};
                    use rayon::prelude::*;
                    use itertools::Itertools;
                    use crate::verification::calibre::{SKY130_DRC_RUNSET_PATH, SKY130_LAYERPROPS_PATH, SKY130_LVS_RULES_PATH};

                    let lvs_path = out_spice(&work_dir, "lvs_schematic");
                    ctx.write_schematic_to_file_for_purpose::<Sram>(
                        &$params,
                        &lvs_path,
                        NetlistPurpose::Lvs,
                    ).expect("failed to write lvs source netlist");
                    let lvs_work_dir = work_dir.join("lvs");
                    let output = run_lvs(&LvsParams{
                        work_dir: &lvs_work_dir,
                        layout_path: &gds_path,
                        layout_cell_name: &$params.name(),
                        source_paths: &[lvs_path],
                        source_cell_name: &$params.name(),
                        rules_path: Path::new(SKY130_LVS_RULES_PATH),
                        layerprops: Some(Path::new(SKY130_LAYERPROPS_PATH)),
                    }).expect("failed to run LVS");
                    assert!(matches!(
                        output.status,
                        calibre::lvs::LvsStatus::Correct
                    ));
                    println!("{}: done running LVS", stringify!($name));


                    let drc_work_dir = work_dir.join("drc");
                    for deck in [
                        "drc", "latchup", "soft", "luRes",
                        // "stress", "fill"
                    ] {
                        let deck_work_dir = drc_work_dir.join(deck);
                        let output = run_drc(&DrcParams {
                            cell_name: &$params.name(),
                            work_dir: &deck_work_dir,
                            layout_path: &gds_path,
                            rules_path: Path::new(&format!("/tools/commercial/skywater/swtech130/skywater-src-nda/s8/V2.0.1/DRC/Calibre/s8_{deck}Rules")),
                            runset_path: (deck == "drc").then(|| Path::new(SKY130_DRC_RUNSET_PATH)),
                            layerprops: Some(Path::new(SKY130_LAYERPROPS_PATH)),
                        }).expect("failed to run DRC");
                        println!("{:?}", output.rule_checks);
                        let mut rulechecks = output.rule_checks.into_iter().filter(|rc| rc.name.starts_with("r_"));
                        assert!(
                            rulechecks.next().is_none(),
                            "DRC must have no rule violations"
                        );
                        println!("{}: done running DRC deck `{}`", stringify!($name), deck);
                    }

                    let pex_path = out_spice(&work_dir, "pex_schematic");
                    let pex_dir = work_dir.join("pex");
                    let _ = std::fs::remove_dir_all(&pex_dir);
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
                    println!("{}: done running PEX", stringify!($name));

                    let seq = TestSequence::Short;
                    let corners = ctx.corner_db();
                    let tt = corners.corner_named("tt").unwrap();
                    let sf = corners.corner_named("sf").unwrap();
                    let fs = corners.corner_named("fs").unwrap();
                    let ss = corners.corner_named("ss").unwrap();
                    let ff = corners.corner_named("ff").unwrap();
                    itertools::iproduct!([1.8], [tt, sf, fs, ss, ff]).collect_vec().into_par_iter().map(|(vdd, corner)| {
                            let params = $params.clone();
                            let pex_netlist = Some((pex_netlist_path.clone(), pex_level));
                            let work_dir = work_dir.clone();
                            let ctx = setup_ctx();
                            let dsn = ctx.run_script::<SramPhysicalDesignScript>(&params).expect("failed to run sram design script");
                            let tb = crate::blocks::sram::testbench::tb_params(params, dsn, vdd, seq, pex_netlist);
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
                                "{}: done simulating in corner {} with Vdd = {}, seq = {}",
                                stringify!($name),
                                corner.name(),
                                vdd,
                                seq,
                            );
                        }).collect::<Vec<_>>();

                    crate::abs::write_abstract(
                        &ctx,
                        &$params,
                        crate::paths::out_lef(&work_dir, &*$params.name()),
                    )
                    .expect("failed to write abstract");
                    println!("{}: done writing abstract", stringify!($name));

                    let sram = ctx.instantiate_layout::<Sram>(&$params).expect("failed to generate layout");
                    let brect = sram.brect();
                    let width = Decimal::new(brect.width(), 3);
                    let height = Decimal::new(brect.height(), 3);
                    [("tt", 25, dec!(1.8)), ("ss", 100, dec!(1.6)), ("ff", -40, dec!(1.95))].into_par_iter().map(|(corner, temp, vdd)| {
                        let verilog_path = verilog_path.clone();
                        let work_dir = work_dir.clone();
                        let pex_netlist_path = pex_netlist_path.clone();
                        let suffix = match corner {
                            "tt" => "tt_025C_1v80",
                            "ss" => "ss_100C_1v60",
                            "ff" => "ff_n40C_1v95",
                            _ => unreachable!(),
                        };
                        let name = format!("{}_{}", $params.name(), suffix);
                        let params = liberate_mx::LibParams::builder()
                            .work_dir(work_dir.join(format!("lib/{suffix}")))
                            .output_file(crate::paths::out_lib(&work_dir, &name))
                            .corner(corner)
                            .width(width)
                            .height(height)
                            .user_verilog(verilog_path)
                            .cell_name(&*$params.name())
                            .num_words($params.num_words())
                            .data_width($params.data_width())
                            .addr_width($params.addr_width())
                            .wmask_width($params.wmask_width())
                            .mux_ratio($params.mux_ratio())
                            .has_wmask(true)
                            .source_paths(vec![pex_netlist_path.clone()])
                            .vdd(vdd)
                            .temp(temp)
                            .build()
                            .unwrap();
                        crate::liberate::generate_sram_lib(&params).expect("failed to write lib");
                        println!("{}: done generating LIB for corner `{}`", stringify!($name), corner);
                    }).collect::<Vec<_>>();
                }

                println!("{}: all tasks complete", stringify!($name));
            }
        };
    }

    test_sram!(test_sram22_64x24m4w8, SRAM22_64X24M4W8, ignore = "slow");
    test_sram!(test_sram22_64x32m4w8, SRAM22_64X32M4W8, ignore = "slow");
    test_sram!(test_sram22_128x16m4w8, SRAM22_128X16M4W8, ignore = "slow");
    test_sram!(test_sram22_128x24m4w8, SRAM22_128X24M4W8, ignore = "slow");
    test_sram!(test_sram22_128x32m4w8, SRAM22_128X32M4W8, ignore = "slow");
    test_sram!(test_sram22_256x8m8w1, SRAM22_256X8M8W1, ignore = "slow");
    test_sram!(test_sram22_256x16m8w8, SRAM22_256X16M8W8, ignore = "slow");
    test_sram!(test_sram22_256x32m4w8, SRAM22_256X32M4W8, ignore = "slow");
    test_sram!(test_sram22_256x64m4w8, SRAM22_256X64M4W8, ignore = "slow");
    test_sram!(test_sram22_256x128m4w8, SRAM22_256X128M4W8, ignore = "slow");
    test_sram!(test_sram22_512x8m8w1, SRAM22_512X8M8W1, ignore = "slow");
    test_sram!(test_sram22_512x32m4w8, SRAM22_512X32M4W8, ignore = "slow");
    test_sram!(test_sram22_512x64m4w8, SRAM22_512X64M4W8, ignore = "slow");
    test_sram!(test_sram22_512x128m4w8, SRAM22_512X128M4W8, ignore = "slow");
    test_sram!(test_sram22_1024x8m8w1, SRAM22_1024X8M8W1, ignore = "slow");
    test_sram!(test_sram22_1024x32m8w8, SRAM22_1024X32M8W8, ignore = "slow");
    test_sram!(test_sram22_1024x64m4w8, SRAM22_1024X64M4W8, ignore = "slow");
    test_sram!(test_sram22_2048x8m8w1, SRAM22_2048X8M8W1, ignore = "slow");
    test_sram!(test_sram22_2048x32m8w8, SRAM22_2048X32M8W8, ignore = "slow");
    test_sram!(test_sram22_4096x8m8w1, SRAM22_4096X8M8W1, ignore = "slow");
    test_sram!(test_sram22_4096x32m8w8, SRAM22_4096X32M8W8, ignore = "slow");
    test_sram!(test_sram22_8192x32m8w8, SRAM22_8192X32M8W8, ignore = "slow");
}
