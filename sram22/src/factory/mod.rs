use crate::{config::TechConfig, error::Result, sky130_config};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

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

pub struct Factory {
    layouts: HashMap<String, Layout>,
    magic: MagicInstance,
    tc: TechConfig,
    work_dir: PathBuf,
    out_dir: PathBuf,
}

pub struct BuildContext<'a> {
    pub tc: &'a TechConfig,
    pub magic: &'a mut MagicInstance,
    pub out_dir: PathBuf,
    pub work_dir: PathBuf,
    pub name: &'a str,
}

pub struct Output {}

impl Factory {
    pub fn default() -> Result<Self> {
        let out_dir = PathBuf::from("/home/rahul/acads/sky130/sram22/_build/");
        let magic_port = portpicker::pick_unused_port().expect("No ports free");

        let magic = MagicInstance::builder()
            .cwd(out_dir.clone())
            .tech("sky130A")
            .port(magic_port)
            .build()
            .unwrap();
        let tc = sky130_config();

        Ok(Self {
            tc,
            magic,
            out_dir,
            layouts: HashMap::new(),
            work_dir: "/tmp/sram22/scratch/".into(),
        })
    }

    pub fn generate_layout<C>(&mut self, name: &str, params: C::Params) -> Result<()>
    where
        C: Component + std::any::Any,
    {
        let work_dir = self.work_dir.join(name);
        std::fs::create_dir_all(&work_dir)?;

        let out_dir = self.out_dir.join("layout");
        std::fs::create_dir_all(&out_dir)?;

        let bc = BuildContext {
            tc: &self.tc,
            magic: &mut self.magic,
            out_dir,
            work_dir,
            name,
        };

        let cell = C::layout(bc, params)?;
        self.layouts.insert(name.to_string(), cell);
        Ok(())
    }

    pub fn get_layout(&self, name: &str) -> Option<Layout> {
        self.layouts.get(name).cloned()
    }

    pub fn magic(&'_ mut self) -> Result<&'_ mut MagicInstance> {
        Ok(&mut self.magic)
    }

    pub fn tc(&mut self) -> &TechConfig {
        &self.tc
    }
}

impl<'a> BuildContext<'a> {
    pub fn layout_from_default_magic(&mut self) -> Result<Layout> {
        let file = format!("{}.mag", self.name);
        let cell = self.magic.load_layout_cell(&file)?;
        let path = self.out_dir.join(file);
        Ok(Layout {
            file: Arc::new(LayoutFile::Magic(path)),
            cell,
        })
    }
}

#[cfg(test)]
mod tests {
    use magic_vlsi::units::Distance;

    use crate::cells::gates::nand::single_height::{Nand2Component, Nand2Params};

    use super::Factory;

    #[test]
    fn test_factory_nand2() -> Result<(), Box<dyn std::error::Error>> {
        let mut f = Factory::default()?;
        f.generate_layout::<Nand2Component>(
            "nand2_test",
            Nand2Params {
                nmos_scale: Distance::from_nm(800),
                height: Distance::from_nm(1_580),
            },
        )?;
        Ok(())
    }
}
