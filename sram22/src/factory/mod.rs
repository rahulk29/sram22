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
    layout_dir: PathBuf,
}

#[derive(derive_builder::Builder)]
pub struct FactoryConfig {
    work_dir: PathBuf,
    out_dir: PathBuf,
    tech_config: TechConfig,
}

impl FactoryConfig {
    pub fn builder() -> FactoryConfigBuilder {
        FactoryConfigBuilder::default()
    }
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
    pub fn from_config(cfg: FactoryConfig) -> Result<Self> {
        let magic_port = portpicker::pick_unused_port().expect("No ports free");

        let layout_dir = cfg.out_dir.join("layout/");
        std::fs::create_dir_all(&cfg.work_dir)?;
        std::fs::create_dir_all(&cfg.out_dir)?;
        std::fs::create_dir_all(&layout_dir)?;

        let magic = MagicInstance::builder()
            .cwd(cfg.out_dir.clone())
            .tech("sky130A")
            .port(magic_port)
            .build()?;

        Ok(Self {
            tc: cfg.tech_config,
            magic,
            out_dir: cfg.out_dir,
            layouts: HashMap::new(),
            work_dir: cfg.work_dir,
            layout_dir,
        })
    }

    pub fn default() -> Result<Self> {
        let out_dir = PathBuf::from("/home/rahul/acads/sky130/sram22/_build/");
        let work_dir = PathBuf::from("/tmp/sram22/scratch/");
        let cfg = FactoryConfig::builder()
            .out_dir(out_dir)
            .work_dir(work_dir)
            .tech_config(sky130_config())
            .build()
            .unwrap();
        Self::from_config(cfg)
    }

    pub fn generate_layout<C>(&mut self, name: &str, params: C::Params) -> Result<()>
    where
        C: Component + std::any::Any,
    {
        let work_dir = self.work_dir.join(name);

        let bc = BuildContext {
            tc: &self.tc,
            magic: &mut self.magic,
            out_dir: self.layout_dir.clone(),
            work_dir,
            name,
        };

        let cell = C::layout(bc, params)?;
        self.layouts.insert(name.to_string(), cell);
        Ok(())
    }

    pub fn include_layout(&mut self, name: &str, f: LayoutFile) -> Result<()> {
        match f {
            LayoutFile::Magic(ref path) => {
                let filename = path.file_name().unwrap();
                let dst = self.layout_dir.join(filename);
                std::fs::copy(path, &dst)?;
                let cell = self.magic.load_layout_cell(dst.to_str().unwrap())?;
                self.layouts.insert(
                    name.to_string(),
                    Layout {
                        file: Arc::new(LayoutFile::Magic(dst)),
                        cell,
                    },
                );
            }
            _ => unimplemented!(),
        };

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

    pub fn out_dir(&self) -> &std::path::Path {
        &self.out_dir
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
