use crate::BUILD_PATH;
use pdkprims::PdkLib;
use std::fmt::Debug;
use std::path::PathBuf;

mod array;
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
mod sense_amp;
mod sram;
mod tmc;
mod wmask_control;

pub(crate) fn panic_on_err<E: Debug>(e: E) -> ! {
    println!("ERROR: {e:?}");
    panic!("ERROR: {e:?}");
}

pub(crate) fn test_gds_path(lib: &PdkLib) -> PathBuf {
    let mut path = PathBuf::from(BUILD_PATH);
    path.push(format!("gds/{}.gds", &lib.lib.name));
    path
}

pub(crate) fn test_verilog_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(BUILD_PATH);
    path.push(format!("verilog/{}.v", name));
    path
}
