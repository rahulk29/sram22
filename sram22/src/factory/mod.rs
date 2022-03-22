use crate::{
    config::TechConfig,
    error::{Result, Sram22Error},
    sky130_config,
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use magic_vlsi::{cell::LayoutCellRef, MagicInstance};
use micro_hdl::backend::spice::SpiceBackend;

pub trait Component {
    type Params: std::any::Any + std::clone::Clone;

    fn schematic(ctx: BuildContext, params: Self::Params) -> micro_hdl::context::ContextTree;
    fn layout(ctx: BuildContext, params: Self::Params) -> crate::error::Result<Layout>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LayoutFile {
    Magic(PathBuf),
    Gds(PathBuf),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Netlist {
    path: PathBuf,
    cell_name: String,
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
    pub file: Arc<LayoutFile>,
    pub cell: LayoutCellRef,
}

pub struct Factory {
    /// Maps fully qualified name to a [`Layout`].
    layouts: HashMap<String, Layout>,
    /// Maps fully qualified name to a [`Netlist`].
    netlists: HashMap<String, Netlist>,
    magic: Arc<Mutex<MagicInstance>>,
    tc: Arc<TechConfig>,
    work_dir: PathBuf,
    out_dir: PathBuf,
    /// Directory where layout (.mag) files are stored
    layout_dir: PathBuf,
    /// Directory where netlist files are stored
    netlist_dir: PathBuf,
    /// Maps short name to fully qualified name.
    remap: HashMap<String, String>,
    /// The fully qualified name for a component is formed
    /// by concatenating `prefix` to its short name:
    ///
    /// ```rust
    /// let prefix = "my_prefix";
    /// let short_name = "my_short_name";
    /// let fqn = format!("{}_{}", prefix, short_name);
    /// assert_eq!(fqn, "my_prefix_my_short_name");
    /// ```
    prefix: String,
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
    pub factory: &'a mut Factory,
    pub tc: Arc<TechConfig>,
    pub magic: MagicHandle<'a>,
    pub out_dir: PathBuf,
    pub work_dir: PathBuf,
    pub name: &'a str,
}

pub struct Output {}

pub struct MagicHandle<'a> {
    pub inner: MutexGuard<'a, MagicInstance>,
}

impl<'a> Deref for MagicHandle<'a> {
    type Target = MagicInstance;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a> DerefMut for MagicHandle<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

impl Factory {
    pub fn from_config(cfg: FactoryConfig) -> Result<Self> {
        let layout_dir = cfg.out_dir.join("layout/");
        let netlist_dir = cfg.out_dir.join("netlist/");

        assert!(cfg.work_dir.is_absolute());
        assert!(cfg.out_dir.is_absolute());

        std::fs::create_dir_all(&cfg.work_dir)?;
        std::fs::create_dir_all(&cfg.out_dir)?;
        std::fs::create_dir_all(&layout_dir)?;
        std::fs::create_dir_all(&netlist_dir)?;

        let magic_port = portpicker::pick_unused_port().expect("No ports free");
        let mut magic = MagicInstance::builder()
            .cwd(layout_dir.clone())
            .tech("sky130A")
            .port(magic_port)
            .build()?;

        // Initial magic settings
        magic.drc_off()?;
        magic.scalegrid(1, 2)?;
        magic.set_snap(magic_vlsi::SnapMode::Internal)?;

        Ok(Self {
            tc: Arc::new(cfg.tech_config),
            magic: Arc::new(Mutex::new(magic)),
            out_dir: cfg.out_dir,
            layouts: HashMap::new(),
            netlists: HashMap::new(),
            remap: HashMap::new(),
            prefix: "sram22".to_string(),
            work_dir: cfg.work_dir,
            layout_dir,
            netlist_dir,
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

    pub fn generate_all<C>(&mut self, name: &str, params: C::Params) -> Result<()>
    where
        C: Component + std::any::Any,
    {
        self.generate_layout::<C>(name, params.clone())?;
        self.generate_schematic::<C>(name, params)?;

        Ok(())
    }

    pub fn generate_schematic<C>(&mut self, name: &str, params: C::Params) -> Result<()>
    where
        C: Component + std::any::Any,
    {
        let work_dir = self.work_dir.join(name);
        std::fs::create_dir_all(&work_dir)?;

        let fqn = self.get_fqn(name);

        let magic = Arc::clone(&self.magic);
        let handle = MagicHandle {
            inner: magic.lock().unwrap(),
        };

        let bc = BuildContext {
            tc: Arc::clone(&self.tc),
            magic: handle,
            out_dir: self.layout_dir.clone(),
            factory: self,
            work_dir,
            name: &fqn,
        };

        let tree = C::schematic(bc, params);
        let path = self.netlist_dir.join(&format!("{}.spice", name));
        let mut netlister = SpiceBackend::with_path(path.clone())?;
        netlister.set_top_name(fqn.clone());
        netlister.netlist(&tree)?;
        let netlist = Netlist {
            path,
            cell_name: fqn.clone(),
        };
        self.netlists.insert(fqn.clone(), netlist);
        self.remap.insert(name.to_string(), fqn);
        Ok(())
    }

    pub fn generate_layout<C>(&mut self, name: &str, params: C::Params) -> Result<()>
    where
        C: Component + std::any::Any,
    {
        let work_dir = self.work_dir.join(name);
        std::fs::create_dir_all(&work_dir)?;

        // Form fully qualified name
        let fqn = self.get_fqn(name);

        let magic = Arc::clone(&self.magic);
        let handle = MagicHandle {
            inner: magic.lock().unwrap(),
        };

        let bc = BuildContext {
            tc: Arc::clone(&self.tc),
            magic: handle,
            out_dir: self.layout_dir.clone(),
            factory: self,
            work_dir,
            name: &fqn,
        };

        let cell = C::layout(bc, params)?;
        self.layouts.insert(fqn.clone(), cell);
        self.remap.insert(name.to_string(), fqn);
        Ok(())
    }

    fn get_fqn(&self, name: &str) -> String {
        format!("{}_{}", &self.prefix, name)
    }

    pub fn include_layout(&mut self, name: &str, f: LayoutFile) -> Result<()> {
        let fqn = self.get_fqn(name);
        let dst_fname = format!("{}.mag", &fqn);
        match f {
            LayoutFile::Magic(ref path) => {
                let dst = self.layout_dir.join(&dst_fname);
                std::fs::copy(path, &dst)?;
                log::info!("copied {:?} to {:?}", path, &dst);
                let cell = self.magic.lock().unwrap().load_layout_cell(&fqn)?;
                log::info!("loaded cell {} with {} ports", &fqn, cell.ports.len());
                self.layouts.insert(
                    fqn.clone(),
                    Layout {
                        file: Arc::new(LayoutFile::Magic(dst)),
                        cell,
                    },
                );
                self.remap.insert(name.to_string(), fqn);
            }
            _ => unimplemented!(),
        };

        Ok(())
    }

    pub fn get_layout(&self, name: &str) -> Option<Layout> {
        self.layouts.get(self.remap.get(name)?).cloned()
    }

    pub fn require_layout(&self, name: &str) -> Result<Layout> {
        let fqn = self
            .remap
            .get(name)
            .ok_or_else(|| Sram22Error::MissingCell(name.to_string()))?;
        self.layouts
            .get(fqn)
            .cloned()
            .ok_or_else(|| Sram22Error::MissingCell(fqn.to_string()))
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
