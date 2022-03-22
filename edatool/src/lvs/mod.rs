use std::path::PathBuf;

use crate::{
    error::Result,
    protos::lvs::{LvsInput, LvsOutput},
};

pub trait Lvs {
    fn lvs(&self, input: LvsInput, work_dir: PathBuf) -> Result<LvsOutput>;
}
