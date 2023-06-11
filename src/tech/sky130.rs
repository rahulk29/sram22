use std::path::PathBuf;

pub const SKY130_DOMAIN: &str = "sky130";
pub const SRAM_SP_CELL: &str = "sram_sp_cell";
pub const SRAM_SP_COLEND: &str = "sky130_fd_bd_sram__sram_sp_colend";
pub const SRAM_SP_CELL_REPLICA: &str = "sram_sp_cell_replica";
pub const OPENRAM_DFF: &str = "openram_dff";
pub const SRAM_CONTROL_SIMPLE: &str = "sramgen_control_simple";
pub const SRAM_CONTROL_REPLICA_V1: &str = "sramgen_control_replica_v1";
pub const SRAM_CONTROL_BUFBUF_16: &str = "control_logic_bufbuf_16";
pub const SRAM_SP_SENSE_AMP: &str = "sramgen_sp_sense_amp";
pub const CONTROL_LOGIC_INV: &str = "control_logic_inv";

pub const BITCELL_HEIGHT: isize = 1580;
pub const BITCELL_WIDTH: isize = 1200;
pub const TAPCELL_WIDTH: isize = 1300;
pub const COLUMN_WIDTH: isize = BITCELL_WIDTH + TAPCELL_WIDTH;

#[inline]
pub fn external_gds_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tech/sky130/gds")
}

#[inline]
pub fn external_spice_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tech/sky130/spice")
}
