use crate::config::SramConfig;
use crate::plan::extract::ExtractionResult;
use crate::schematic::decoder::DecoderTree;
use anyhow::{anyhow, Result};

pub mod extract;

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
pub struct SramPlan {
    pub decoder: DecoderTree,
}

pub fn generate_plan(
    _extraction_result: ExtractionResult,
    _config: SramConfig,
) -> Result<SramPlan> {
    Err(anyhow!("unimplemented"))
}
