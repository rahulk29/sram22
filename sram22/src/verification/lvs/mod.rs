use crate::error::Result;
use std::path::PathBuf;

pub struct LvsInput<S> {
    pub netlist: PathBuf,
    pub layout: PathBuf,
    pub netlist_cell: String,
    pub layout_cell: String,
    pub work_dir: PathBuf,
    pub opts: S,
}

pub struct LvsOutput<E> {
    pub ok: bool,
    pub errors: Vec<E>,
}

pub trait Lvs<S, E> {
    fn lvs(&self, input: LvsInput<S>) -> Result<LvsOutput<E>>;
}

pub struct LvsOpts {}

pub struct LvsError {
    pub msg: String,
}
