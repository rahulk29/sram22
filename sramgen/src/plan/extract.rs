use anyhow::Result;

/// Technology-specific parameters and standard cells to extract values relevant to SRAM
/// generation from.
pub struct ExtractionParameters {}

/// Result of extraction.
pub struct ExtractionResult {}

/// Extract values relevant to SRAM generation from the provided technology and standard cells.
pub fn extract(_params: ExtractionParameters) -> Result<ExtractionResult> {
    Ok(ExtractionResult {})
}
