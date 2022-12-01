use crate::cli::{StepContext, StepKey};
use crate::config::sram::{ControlMode, SramConfig, SramParams};
use crate::layout::sram::draw_sram;
use crate::paths::{out_bin, out_gds, out_sram, out_verilog};
use crate::plan::extract::ExtractionResult;
use crate::schematic::sram::sram;
use crate::schematic::{generate_netlist, save_modules};
use crate::verilog::save_1rw_verilog;
use crate::{clog2, Result};
use anyhow::{bail, Context};
use pdkprims::tech::sky130;
use std::path::Path;

pub mod extract;

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
pub struct SramPlan {
    pub sram_params: SramParams,
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
    } = config;

    if control != ControlMode::Simple && control != ControlMode::ReplicaV1 {
        bail!(
            "Only `ControlMode::Simple` and `ControlMode::ReplicaV1` are supported at the moment"
        );
    }
    if data_width % write_size != 0 {
        bail!("Data width must be a multiple of write size");
    }

    let name = out_sram(config);
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
            name,
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

pub fn execute_plan(
    work_dir: impl AsRef<Path>,
    plan: &SramPlan,
    mut ctx: Option<&mut StepContext>,
) -> Result<()> {
    std::fs::create_dir_all(work_dir.as_ref())?;

    let modules = sram(&plan.sram_params);

    let name = &plan.sram_params.name;

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules).with_context(|| "Error saving netlist binaries")?;

    generate_netlist(&bin_path, &work_dir)
        .with_context(|| "Error converting netlists to SPICE format")?;

    if let Some(ctx) = ctx.as_mut() {
        ctx.finish(StepKey::GenerateNetlist);
    }

    let mut lib = sky130::pdk_lib(name)?;
    draw_sram(&mut lib, &plan.sram_params).with_context(|| "Error generating SRAM layout")?;

    let gds_path = out_gds(&work_dir, name);
    lib.save_gds(&gds_path)
        .with_context(|| "Error saving SRAM GDS")?;

    if let Some(ctx) = ctx.as_mut() {
        ctx.finish(StepKey::GenerateLayout);
    }

    let verilog_path = out_verilog(&work_dir, name);
    save_1rw_verilog(&verilog_path, &plan.sram_params)
        .with_context(|| "Error generating or saving Verilog model")?;

    if let Some(ctx) = ctx.as_mut() {
        ctx.finish(StepKey::GenerateVerilog);
    }

    #[cfg(feature = "abstract_lef")]
    {
        let lef_path = crate::paths::out_lef(&work_dir, name);
        crate::abs::run_sram_abstract(&work_dir, name, &lef_path, &gds_path, &verilog_path)?;

        if let Some(ctx) = ctx {
            ctx.finish(StepKey::GenerateLef);
        }
    }

    #[cfg(feature = "liberate_mx")]
    {
        use crate::verification::{source_files, VerificationTask};
        use liberate_mx::LibParams;

        let source_paths = source_files(
            work_dir.as_ref(),
            &plan.sram_params.name,
            VerificationTask::SpectreSim,
            plan.sram_params.control,
        );
        let params = LibParams::builder()
            .work_dir(work_dir.as_ref().join("lib"))
            .save_dir(work_dir.as_ref())
            .corner("tt")
            .cell_name(&plan.sram_params.name)
            .num_words(plan.sram_params.num_words)
            .data_width(plan.sram_params.data_width)
            .addr_width(plan.sram_params.addr_width)
            .wmask_width(plan.sram_params.wmask_width)
            .mux_ratio(plan.sram_params.mux_ratio)
            .source_paths(source_paths)
            .build()?;

        crate::liberate::generate_sram_lib(&params)?;
    }
    Ok(())
}
