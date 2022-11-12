use crate::schematic::decoder::DecoderTree;

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
pub struct SramPlan {
    pub decoder: DecoderTree,
}
