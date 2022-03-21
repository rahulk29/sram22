use crate::{config::TechConfig, error::Result};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use magic_vlsi::{cell::LayoutCellRef, MagicInstance};

pub trait Component {
    type Params: std::any::Any;

    fn schematic(ctx: BuildContext, params: Self::Params) -> micro_hdl::context::ContextTree;
    fn layout(ctx: BuildContext, params: Self::Params) -> crate::error::Result<Layout>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LayoutFile {
    Magic(PathBuf),
    Gds(PathBuf),
}

impl LayoutFile {
    pub fn path(&self) -> &std::path::Path {
        match self {
            LayoutFile::Gds(ref path) => path,
            LayoutFile::Magic(ref path) => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    file: Arc<LayoutFile>,
    cell: LayoutCellRef,
}

type FactoryRef = Arc<Mutex<Factory>>;

// Requirements:
// user friendly names can be mapped to fully qualified names
//     (eg. "bitcell" -> "sky130_sram_sp_cell")
// strong typing for params
// strong typing for outputs
//

pub struct Factory {
    layouts: HashMap<String, Layout>,
    magic: MagicInstance,
    tc: TechConfig,
}

pub struct BuildContext<'a> {
    pub factory: &'a mut Factory,
    pub out_dir: PathBuf,
    pub work_dir: PathBuf,
    pub name: &'a str,
}

pub struct Output {}

impl Factory {
    pub fn generate_layout<C>(&mut self, name: &str, params: C::Params) -> Result<()>
    where
        C: Component + std::any::Any,
    {
        let bc = BuildContext {
            factory: self,
            out_dir: "/tmp/".into(),
            work_dir: "/tmp/".into(),
            name,
        };

        let cell = C::layout(bc, params)?;
        self.layouts.insert(name.to_string(), cell);
        Ok(())
    }

    pub fn get_layout(&self, name: &str) -> Option<Layout> {
        self.layouts.get(name).map(|x| x.clone())
    }

    pub fn magic<'a>(&'a mut self) -> Result<&'a mut MagicInstance> {
        Ok(&mut self.magic)
    }

    pub fn tc(&self) -> &TechConfig {
        &self.tc
    }
}
