use std::path::PathBuf;

use crate::BUILD_PATH;

mod bitcells;
mod col_inv;
mod control;
mod decoder;
mod dff;
mod dout_buffer;
mod edge_detector;
mod gate;
mod guard_ring;
mod inv_chain;
mod latch;
mod mux;
mod precharge;
mod sense_amp;
mod sram;
mod tmc;
mod wl_driver;
mod wmask_control;

pub(crate) fn test_work_dir(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(name)
}
