use std::{
    any::Any,
    fmt::Debug,
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub type FactoryFn = fn(ctx: &BuildContext);

pub trait FactoryParams {
    // TODO
}

type FactoryRef = Arc<RwLock<Factory>>;

// Requirements:
// user friendly names can be mapped to fully qualified names
//     (eg. "bitcell" -> "sky130_sram_sp_cell")
// strong typing for params
// strong typing for outputs
//

pub struct Factory {}

#[derive(Clone)]
pub struct FactoryHandle {
    inner: FactoryRef,
}

pub struct BuildContext {
    pub factory: FactoryHandle,
    pub out_dir: PathBuf,
    pub work_dir: PathBuf,
    pub name: String,
}

pub enum Target {
    Netlist,
    Layout,
    Lvs,
    Drc,
    Sim,
}

pub struct Output {}

impl Factory {
    pub fn get_prerequisite(&mut self, name: &str, target: Target, p: Box<dyn Any>) {}
}
