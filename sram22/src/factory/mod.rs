use crate::{
    error::{Result, Sram22Error},
    sky130_config,
};
use edatool::{
    lvs::Lvs,
    protos::lvs::{LvsInput, LvsTool},
};
use layout21::{utils::Ptr, raw::Cell};
use pdkprims::Pdk;
use std::{
    collections::HashMap,
    path::{PathBuf, Path},
    sync::{Arc, Mutex, MutexGuard},
};

use micro_hdl::backend::spice::SpiceBackend;

pub trait Component {
    type Params: std::any::Any + std::clone::Clone;

    fn schematic(ctx: BuildContext, params: Self::Params) -> micro_hdl::context::ContextTree;
    fn layout(ctx: BuildContext, params: Self::Params) -> crate::error::Result<Layout>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Netlist {
    path: PathBuf,
    cell_name: String,
}


pub type Layout = Ptr<Cell>;

pub struct Factory {
    /// Maps fully qualified name to a [`Layout`].
    layouts: HashMap<String, Layout>,
    /// Maps fully qualified name to a [`Netlist`].
    netlists: HashMap<String, Netlist>,
    pdk: Arc<Pdk>,
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
}

impl FactoryConfig {
    pub fn builder() -> FactoryConfigBuilder {
        FactoryConfigBuilder::default()
    }
}

pub struct BuildContext<'a> {
    pub factory: &'a mut Factory,
    pub pdk: Arc<Pdk>,
    pub out_dir: PathBuf,
    pub work_dir: PathBuf,
    pub name: &'a str,
}

pub struct Output {}


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

        let pdk = pdkprims::tech::sky130::pdk()?;

        Ok(Self {
            pdk: Arc::new(pdk),
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
        self.lvs(name)?;

        Ok(())
    }

    fn lvs(&mut self, name: &str) -> Result<()> {
        let fqn = self.get_fqn(name);
        let layout = self.get_layout(name).unwrap();
        let netlist = self.get_schematic(name).unwrap();
        let work_dir = self.work_dir.join("lvs_dir").join(name);
        let lvs = edatool::plugins::netgen_lvs::NetgenLvs::new();
        let lvs_output = lvs
            .lvs(
                todo!(),
                // LvsInput {
                //     netlist_path: netlist.path.into_os_string().into_string().unwrap(),
                //     layout_path: layout
                //         .file
                //         .path()
                //         .to_owned()
                //         .into_os_string()
                //         .into_string()
                //         .unwrap(),
                //     netlist_cell: fqn.clone(),
                //     layout_cell: fqn,
                //     tech: "sky130".into(),
                //     tool: LvsTool::MagicNetgen as i32,
                //     options: HashMap::default(),
                // },
                work_dir,
            )
            .map_err(|e| Sram22Error::Unknown(Box::new(e)))?;

        if !lvs_output.matches {
            log::warn!("cell {} did not pass LVS", name);
        }
        Ok(())
    }

    pub fn generate_schematic<C>(&mut self, name: &str, params: C::Params) -> Result<()>
    where
        C: Component + std::any::Any,
    {
        let work_dir = self.work_dir.join(name);
        std::fs::create_dir_all(&work_dir)?;

        let fqn = self.get_fqn(name);

        let bc = BuildContext {
            pdk: Arc::clone(&self.pdk),
            out_dir: self.layout_dir.clone(),
            factory: self,
            work_dir,
            name: &fqn,
        };

        let tree = C::schematic(bc, params);
        let path = self.netlist_dir.join(&format!("{}.spice", name));
        let mut netlister = SpiceBackend::with_path(path.clone()).map_err(|e| Sram22Error::Unknown(e))?;
        netlister.set_top_name(fqn.clone());
        netlister.netlist(&tree).map_err(|e| Sram22Error::Unknown(e))?;
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

        let bc = BuildContext {
            pdk: Arc::clone(&self.pdk),
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

    pub fn include_layout(&mut self, name: &str, f: impl AsRef<Path>) -> Result<()> {
        let fqn = self.get_fqn(name);
        let dst_fname = format!("{}.mag", &fqn);

        todo!();

        Ok(())
    }

    pub fn get_layout(&self, name: &str) -> Option<Layout> {
        self.layouts.get(self.remap.get(name)?).cloned()
    }

    pub fn get_schematic(&self, name: &str) -> Option<Netlist> {
        self.netlists.get(self.remap.get(name)?).cloned()
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

    pub fn out_dir(&self) -> &std::path::Path {
        &self.out_dir
    }
}

