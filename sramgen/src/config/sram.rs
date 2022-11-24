use crate::config::ControlMode;

pub struct SramParams {
    pub name: String,
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
