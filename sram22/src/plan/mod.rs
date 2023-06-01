use crate::cli::progress::StepContext;
use crate::config::sram::SramConfig;
use crate::paths::{out_gds, out_spice, out_verilog};
use crate::plan::extract::ExtractionResult;
use crate::v2::sram::verilog::save_1rw_verilog;
use crate::v2::sram::{Sram, SramParams};
use crate::{clog2, setup_ctx, Result};
use anyhow::bail;
use std::collections::{HashSet, HashMap};
use std::path::Path;
use substrate::schematic::netlist::NetlistPurpose;
use substrate::verification::pex::PexInput;

pub mod extract;

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
pub struct SramPlan {
    pub sram_params: SramParams,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum TaskKey {
    GeneratePlan,
    GenerateNetlist,
    GenerateLayout,
    GenerateVerilog,
    #[cfg(feature = "commercial")]
    GenerateLef,
    #[cfg(feature = "commercial")]
    RunDrc,
    #[cfg(feature = "commercial")]
    RunLvs,
    #[cfg(feature = "commercial")]
    RunPex,
    #[cfg(feature = "commercial")]
    GenerateLib,
    #[cfg(feature = "commercial")]
    RunSpectre,
    #[cfg(feature = "commercial")]
    All,
}

pub struct ExecutePlanParams<'a> {
    pub work_dir: &'a Path,
    pub plan: &'a SramPlan,
    pub tasks: &'a HashSet<TaskKey>,
    pub ctx: Option<&'a mut StepContext>,
    #[cfg(feature = "commercial")]
    pub pex_level: Option<calibre::pex::PexLevel>,
}

pub fn generate_plan(
    _extraction_result: ExtractionResult,
    config: &SramConfig,
) -> Result<SramPlan> {
    let &SramConfig {
        num_words,
        data_width,
        mux_ratio,
        write_size,
        control,
        ..
    } = config;

    if data_width % write_size != 0 {
        bail!("Data width must be a multiple of write size");
    }

    let rows = (num_words / mux_ratio) as usize;
    let cols = (data_width * mux_ratio) as usize;
    let row_bits = clog2(rows);
    let col_bits = clog2(cols);
    let col_select_bits = clog2(mux_ratio as usize);
    let wmask_width = (data_width / write_size) as usize;
    let mux_ratio = mux_ratio as usize;
    let num_words = num_words as usize;
    let data_width = data_width as usize;
    let addr_width = clog2(num_words);

    Ok(SramPlan {
        sram_params: SramParams {
            wmask_width,
            row_bits,
            col_bits,
            col_select_bits,
            rows,
            cols,
            mux_ratio,
            num_words,
            data_width,
            addr_width,
            control,
        },
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
    save_1rw_verilog(&verilog_path, name.as_str(), &plan.sram_params)
        .expect("failed to write behavioral model");
    try_finish_task!(ctx, TaskKey::GenerateVerilog);

    #[cfg(feature = "commercial")]
    {
        try_execute_task!(
            params.tasks,
            TaskKey::GenerateLef,
            crate::abs::run_abstract(
                work_dir,
                name,
                crate::paths::out_lef(work_dir, name),
                &gds_path,
                &verilog_path
            )?,
            ctx
        );

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
        let pex_netlist_path = params
            .pex_level
            .map(|pex_level| crate::paths::out_pex(work_dir, name, pex_level));
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
                    layout_path: gds_path.clone(),
                    layout_cell_name: name.clone(),
                    layout_format: substrate::layout::LayoutFormat::Gds,
                    source_paths: vec![pex_source_path],
                    source_cell_name: name.clone(),
                    pex_netlist_path: pex_out_path,
                    opts,
                })?;
            },
            ctx
        );

        /*
        try_execute_task!(
            params.tasks,
            TaskKey::RunSpectre,
            crate::verification::spectre::run_sram_spectre(&plan.sram_params, work_dir, name)?,
            ctx
        );
        */

        try_execute_task!(
            params.tasks,
            TaskKey::GenerateLib,
            {
                use substrate::schematic::netlist::NetlistPurpose;

                let (source_path, lib_file) = if let Some(pex_netlist_path) = pex_netlist_path {
                    if !pex_netlist_path.exists() {
                        bail!("PEX netlist not found at path `{:?}`", pex_netlist_path);
                    }
                    (
                        pex_netlist_path,
                        work_dir.join(format!(
                            "{}_tt_025C_1v80.{}.lib",
                            params.plan.sram_params.name(),
                            params.pex_level.unwrap()
                        )),
                    )
                } else {
                    let timing_spice_path = out_spice(work_dir, "timing_schematic");
                    sctx.write_schematic_to_file_for_purpose::<Sram>(
                        &plan.sram_params,
                        &timing_spice_path,
                        NetlistPurpose::Timing,
                    )
                    .expect("failed to write timing schematic");
                    (
                        timing_spice_path,
                        work_dir.join(format!(
                            "{}_tt_025C_1v80.schematic.lib",
                            params.plan.sram_params.name()
                        )),
                    )
                };

                let params = liberate_mx::LibParams::builder()
                    .work_dir(work_dir.join("lib"))
                    .output_file(lib_file)
                    .corner("tt")
                    .cell_name(name.as_str())
                    .num_words(plan.sram_params.num_words)
                    .data_width(plan.sram_params.data_width)
                    .addr_width(plan.sram_params.addr_width)
                    .wmask_width(plan.sram_params.wmask_width)
                    .mux_ratio(plan.sram_params.mux_ratio)
                    .has_wmask(true)
                    .source_paths(vec![source_path])
                    .build()
                    .unwrap();
                crate::liberate::generate_sram_lib(&params).expect("failed to write lib");
            },
            ctx
        );
    }
    Ok(())
}
