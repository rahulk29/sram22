use crate::error::Result;
use std::path::PathBuf;

pub struct PexInput<S> {
    pub layout: PathBuf,
    pub layout_cell: String,
    pub work_dir: PathBuf,
    pub tech: String,
    pub opts: S,
}

pub struct PexOutput {
    pub netlist: PathBuf,
}

pub trait Pex<S> {
    fn pex(input: PexInput<S>) -> Result<PexOutput>;
}

pub struct PexOpts {}
