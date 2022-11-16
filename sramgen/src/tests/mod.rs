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
    let mut path = PathBuf::from(BUILD_PATH);
    path.push(format!("gds/{}.gds", name));
    path
}

pub(crate) fn test_verilog_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(BUILD_PATH);
    path.push(format!("verilog/{}.v", name));
    path
}
