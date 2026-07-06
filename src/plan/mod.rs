use crate::blocks::sram::{Sram, SramConfig, SramParams};
use crate::cli::progress::StepContext;
use crate::paths::{out_gds, out_spice, out_verilog};
use crate::verilog::save_1rw_verilog;
use crate::{setup_ctx, Result};
use anyhow::bail;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Serializes Liberate MX characterization across concurrently-building SRAM
/// configurations, so at most one configuration's corner simulations (and
/// thus its Liberate MX license usage) are in flight at a time. See the
/// `GenerateLibMx` task below.
#[cfg(feature = "commercial")]
static LIBERATE_MX_SLOT: Mutex<()> = Mutex::new(());

/// Embedded timing characterization data indexed by (num_words, mux_ratio, write_size).
/// Each entry corresponds to `timingdata/{num_words}m{mux_ratio}w{write_size}.json`.
/// Add a new row here when a new characterization dataset is available.
static TIMING_DATA: &[(usize, usize, usize, &[u8])] = &[
    (64,   4, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/64m4w8.json"))),
    (128,  4, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/128m4w8.json"))),
    (128,  8, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/128m8w8.json"))),
    (256,  4, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/256m4w8.json"))),
    (256,  8, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/256m8w8.json"))),
    (512,  4, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/512m4w8.json"))),
    (512,  8, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/512m8w8.json"))),
    (1024, 4, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/1024m4w8.json"))),
    (1024, 8, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/1024m8w8.json"))),
    (2048, 4, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/2048m4w8.json"))),
    (2048, 8, 8, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/timingdata/2048m8w8.json"))),
];

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
#[derive(Clone)]
pub struct SramPlan {
    pub sram_params: SramParams,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum TaskKey {
    GeneratePlan,
    GenerateNetlist,
    GenerateLayout,
    GenerateVerilog,
    GenerateLef,
    #[cfg(feature = "commercial")]
    RunDrc,
    #[cfg(feature = "commercial")]
    RunLvs,
    #[cfg(feature = "commercial")]
    RunPex,
    GenerateLib,
    #[cfg(feature = "commercial")]
    GenerateLibMx,
    #[cfg(feature = "commercial")]
    All,
}

pub struct ExecutePlanParams<'a> {
    pub work_dir: &'a Path,
    pub plan: &'a SramPlan,
    pub tasks: Arc<HashSet<TaskKey>>,
    pub ctx: Option<&'a mut StepContext>,
    #[cfg(feature = "commercial")]
    pub pex_level: Option<calibre::pex::PexLevel>,
}

pub fn generate_plan(config: &SramConfig) -> Result<SramPlan> {
    let &SramConfig {
        num_words,
        data_width,
        mux_ratio,
        write_size,
        ..
    } = config;

    if data_width % write_size != 0 {
        bail!("Data width must be a multiple of write size");
    }

    let params = SramParams::new(write_size, mux_ratio, num_words, data_width);

    if 2usize.pow(params.row_bits().try_into().unwrap()) != params.rows() || params.rows() < 16 {
        bail!("The number of rows (num words / mux ratio) must be a power of 2 greater than or equal to 16");
    }

    if params.cols() < 16 {
        bail!("The number of columns (data width * mux ratio) must be at least 16");
    }

    Ok(SramPlan {
        sram_params: params,
    })
}

macro_rules! try_finish_task {
    ( $ctx:expr, $task:expr ) => {
        if let Some(ctx) = $ctx.as_mut() {
            ctx.finish($task);
        }
    };
}

#[cfg(feature = "commercial")]
macro_rules! try_execute_task {
    ( $tasks:expr, $task:expr, $body:expr, $ctx:expr) => {
        if $tasks.contains(&$task) || $tasks.contains(&TaskKey::All) {
            $body;
            try_finish_task!($ctx, $task);
        }
    };
}

pub fn execute_plan(params: ExecutePlanParams) -> Result<()> {
    let ExecutePlanParams {
        work_dir,
        plan,
        mut ctx,
        ..
    } = params;

    std::fs::create_dir_all(work_dir)?;

    let name = &plan.sram_params.name();
    let sctx = setup_ctx();

    let spice_path = out_spice(work_dir, name);
    sctx.write_schematic_to_file::<Sram>(&plan.sram_params, &spice_path)
        .expect("failed to write schematic");
    try_finish_task!(ctx, TaskKey::GenerateNetlist);

    let gds_path = out_gds(work_dir, name);
    sctx.write_layout::<Sram>(&plan.sram_params, &gds_path)
        .expect("failed to write layout");
    try_finish_task!(ctx, TaskKey::GenerateLayout);

    let verilog_path = out_verilog(work_dir, name);
    save_1rw_verilog(&verilog_path, &plan.sram_params).expect("failed to write behavioral model");
    try_finish_task!(ctx, TaskKey::GenerateVerilog);

    crate::abs::write_abstract(
        &sctx,
        &plan.sram_params,
        crate::paths::out_lef(work_dir, name),
    )
    .expect("failed to write abstract");
    try_finish_task!(ctx, TaskKey::GenerateLef);

    #[cfg(feature = "commercial")]
    {
        use std::collections::HashMap;

        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;
        use subgeom::bbox::BoundBox;
        use substrate::schematic::netlist::NetlistPurpose;
        use substrate::verification::pex::PexInput;

        try_execute_task!(
            params.tasks,
            TaskKey::RunDrc,
            {
                let drc_work_dir = work_dir.join("drc");
                let output = sctx
                    .write_drc::<Sram>(&plan.sram_params, drc_work_dir)
                    .expect("failed to run DRC");
                assert!(
                    matches!(
                        output.summary,
                        substrate::verification::drc::DrcSummary::Pass
                    ),
                    "DRC failed"
                );
            },
            ctx
        );
        try_execute_task!(
            params.tasks,
            TaskKey::RunLvs,
            {
                let lvs_work_dir = work_dir.join("lvs");
                let output = sctx
                    .write_lvs::<Sram>(&plan.sram_params, lvs_work_dir)
                    .expect("failed to run LVS");
                assert!(
                    matches!(
                        output.summary,
                        substrate::verification::lvs::LvsSummary::Pass
                    ),
                    "LVS failed"
                );
            },
            ctx
        );

        if params.pex_level.is_none() && params.tasks.contains(&TaskKey::RunPex) {
            bail!("Must specify a PEX level when running PEX");
        }
        let pex_dir = work_dir.join("pex");
        let pex_source_path = out_spice(&pex_dir, "schematic");
        let pex_out_path = out_spice(&pex_dir, "schematic.pex");

        try_execute_task!(
            params.tasks,
            TaskKey::RunPex,
            {
                sctx.write_schematic_to_file_for_purpose::<Sram>(
                    &plan.sram_params,
                    &pex_source_path,
                    NetlistPurpose::Pex,
                )?;
                let mut opts = HashMap::with_capacity(1);
                opts.insert("level".into(), params.pex_level.unwrap().as_str().into());

                sctx.run_pex(PexInput {
                    work_dir: pex_dir,
                    layout_path: gds_path,
                    layout_cell_name: name.clone(),
                    layout_format: substrate::layout::LayoutFormat::Gds,
                    source_paths: vec![pex_source_path],
                    source_cell_name: name.clone(),
                    pex_netlist_path: pex_out_path.clone(),
                    opts,
                    ground_net: "vss".to_string(),
                })?;
            },
            ctx
        );

        let sram_params = plan.sram_params.clone();
        try_execute_task!(
            params.tasks,
            TaskKey::GenerateLibMx,
            {
                use substrate::schematic::netlist::NetlistPurpose;

                let source_path = if params.pex_level.is_some() {
                    if !pex_out_path.exists() {
                        bail!("PEX netlist not found at path `{:?}`", pex_out_path);
                    }
                    pex_out_path
                } else {
                    let timing_spice_path = out_spice(work_dir, "timing_schematic");
                    sctx.write_schematic_to_file_for_purpose::<Sram>(
                        &sram_params,
                        &timing_spice_path,
                        NetlistPurpose::Timing,
                    )
                    .expect("failed to write timing schematic");
                    timing_spice_path
                };

                let sram = sctx
                    .instantiate_layout::<Sram>(&sram_params)
                    .expect("failed to generate layout");
                let brect = sram.brect();
                let width = Decimal::new(brect.width(), 3);
                let height = Decimal::new(brect.height(), 3);

                // Only one SRAM configuration's Liberate MX characterization
                // runs at a time, even though SRAMs otherwise build (plan,
                // netlist, layout, Verilog, LEF, interpolated LIB) fully
                // concurrently. Each configuration checks out 3 licenses (one
                // per corner) from a small shared pool; letting many
                // configurations launch simultaneously exhausts it.
                let _liberate_slot = LIBERATE_MX_SLOT
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);

                let mut handles = Vec::new();
                for (corner, temp, vdd) in [
                    ("tt", 25, dec!(1.8)),
                    ("ss", 100, dec!(1.6)),
                    ("ff", -40, dec!(1.95)),
                ] {
                    let verilog_path = verilog_path.clone();
                    let work_dir = std::path::PathBuf::from(work_dir);
                    let source_path = source_path.clone();
                    let sram_params = sram_params.clone();
                    handles.push(std::thread::spawn(move || {
                        let suffix = match corner {
                            "tt" => "tt_025C_1v80",
                            "ss" => "ss_100C_1v60",
                            "ff" => "ff_n40C_1v95",
                            _ => unreachable!(),
                        };
                        let name = format!("{}_{}", sram_params.name(), suffix);
                        let lib_params = liberate_mx::LibParams::builder()
                            .work_dir(work_dir.join(format!("lib/{suffix}")))
                            .output_file(crate::paths::out_lib(&work_dir, &name))
                            .corner(corner)
                            .width(width)
                            .height(height)
                            .user_verilog(verilog_path)
                            .cell_name(&*sram_params.name())
                            .num_words(sram_params.num_words())
                            .data_width(sram_params.data_width())
                            .addr_width(sram_params.addr_width())
                            .wmask_width(sram_params.wmask_width())
                            .mux_ratio(sram_params.mux_ratio())
                            .has_wmask(true)
                            .source_paths(vec![source_path])
                            .vdd(vdd)
                            .temp(temp)
                            .build()
                            .unwrap();
                        crate::liberate::generate_sram_lib(&lib_params)
                            .expect("failed to write lib");
                    }));
                }
                let handles: Vec<_> = handles.into_iter().map(|handle| handle.join()).collect();
                handles
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .expect("failed to join threads");
            },
            ctx
        );
    }

    if params.tasks.contains(&TaskKey::GenerateLib) {
        use crate::lib_gen::{LibGenParams, LookupModel, PvtCorner};
        if plan.sram_params.data_width() > 128 {
            anyhow::bail!(
                "lib generation requires data_width ≤ 128 in non-commercial mode (got {})",
                plan.sram_params.data_width()
            );
        } else {
            let nw = plan.sram_params.num_words();
            let mx = plan.sram_params.mux_ratio();
            let ws = plan.sram_params.wmask_granularity();
            let json_bytes: &[u8] = TIMING_DATA.iter()
                .find(|(n, m, w, _)| *n == nw && *m == mx && *w == ws)
                .map(|(_, _, _, b)| *b)
                .ok_or_else(|| anyhow::anyhow!(
                    "no timing data for {}m{}w{} — add timingdata/{}m{}w{}.json to the repo",
                    nw, mx, ws, nw, mx, ws
                ))?;
            for pvt in [PvtCorner::tt(), PvtCorner::ss(), PvtCorner::ff()] {
                let model = LookupModel::from_json(json_bytes, &pvt.name)?;
                let suffix = pvt.file_suffix();
                let lib_name = format!("{}_{}", name, suffix);
                let lib_path = crate::paths::out_lib(work_dir, &lib_name);
                crate::lib_gen::generate_sram_lib(&LibGenParams {
                    sram: &plan.sram_params,
                    pvt,
                    model: &model,
                    output: lib_path,
                })?;
            }
            try_finish_task!(ctx, TaskKey::GenerateLib);
        }
    }

    Ok(())
}
