use crate::BUILD_PATH;
use std::fmt::Debug;
use std::path::PathBuf;

mod bitcells;
mod col_inv;
mod control;
mod decoder;
mod dff;
mod dout_buffer;
mod gate;
mod guard_ring;
mod latch;
mod mux;
mod precharge;
mod rbl;
mod sense_amp;
mod sram;
mod tmc;
mod wl_driver;
mod wmask_control;

pub(crate) fn panic_on_err<E: Debug>(e: E) -> ! {
    println!("ERROR: {e:?}");
    panic!("ERROR: {e:?}");
}

pub(crate) fn test_gds_path(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(format!("gds/{}.gds", name))
}

pub(crate) fn test_verilog_path(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(format!("verilog/{}.v", name))
}
