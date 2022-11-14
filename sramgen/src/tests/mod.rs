use pdkprims::PdkLib;
use std::fmt::Debug;
use std::path::PathBuf;

mod array;
mod bank;
mod col_inv;
mod control;
mod decoder;
mod dff;
mod dout_buffer;
mod gate;
mod guard_ring;
mod latch;
mod mux;
mod power;
mod precharge;
mod sense_amp;
mod tmc;
mod wmask_control;

pub const TEST_BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");

pub fn test_gds_path(lib: &PdkLib) -> PathBuf {
    let mut path = PathBuf::from(TEST_BUILD_PATH);
    path.push(format!("gds/{}.gds", &lib.lib.name));
    path
}

pub fn test_lef_path(lib: &PdkLib) -> PathBuf {
    let mut path = PathBuf::from(TEST_BUILD_PATH);
    path.push(format!("lef/{}.lef", &lib.lib.name));
    path
}

pub fn panic_on_err<E: Debug>(e: E) -> ! {
    println!("ERROR: {e:?}");
    panic!("ERROR: {e:?}");
}
