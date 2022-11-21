use crate::config::{sram::SramParams, SramConfig};
use crate::layout::sram::draw_sram;
use crate::out_bin;
use crate::plan::extract::ExtractionResult;
use crate::schematic::sram::sram;
use crate::schematic::{generate_netlist, save_modules};
use crate::tech::sky130;
use crate::{clog2, Result};
use anyhow::Context;
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

    let name = format!("sramgen_sram_{data_width}x{num_words}m{mux_ratio}w{write_size}_simple");
    let rows = (num_words / mux_ratio) as usize;
    let cols = (data_width * mux_ratio) as usize;
    let row_bits = clog2(rows);
    let col_bits = clog2(cols);
    let col_mask_bits = clog2(mux_ratio as usize);
    let wmask_groups = (data_width / write_size) as usize;
    let mux_ratio = mux_ratio as usize;
    let num_words = num_words as usize;
    let data_width = data_width as usize;
    let addr_width = clog2(num_words);

    Ok(SramPlan {
        sram_params: SramParams {
            name,
            wmask_groups,
            row_bits,
            col_bits,
            rows,
            cols,
            mux_ratio,
            num_words,
            data_width,
            addr_width,
        },
    })
}

pub fn execute_plan(work_dir: impl AsRef<Path>, plan: &SramPlan) -> Result<()> {
    let modules = sram(plan.sram_params);

    let name = &plan.sram_params.name;

    save_modules(out_bin(work_dir, name), name, modules)
        .with_context(|| "Error saving netlist binaries")?;

    generate_netlist(&name).with_context(|| "Error converting netlists to SPICE format")?;

    let mut lib = sky130::pdk_lib(&name)?;
    draw_sram_bank(
        &mut lib,
        SramBankParams {
            name: name.clone(),
            rows,
            cols,
            mux_ratio,
            wmask_groups,
        },
    )
    .with_context(|| "Error generating SRAM layout")?;
    Ok(())
}
