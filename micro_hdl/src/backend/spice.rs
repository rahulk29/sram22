use crate::context::ContextTree;

use crate::mos::sky130_mos_name;
use crate::primitive::mos::{Flavor, Intent};
use crate::Module;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

pub type MosNameFn = fn(Flavor, Intent) -> String;

pub struct SpiceBackend<T>
where
    T: std::io::Write,
{
    generated: HashMap<TypeId, HashSet<String>>,
    out: T,
    mos_name_fn: MosNameFn,
    top_name: Option<String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NetlistOpts {
    pub top: bool,
}

impl SpiceBackend<File> {
    pub fn with_path(
        out: impl AsRef<Path>,
    ) -> Result<SpiceBackend<File>, Box<dyn std::error::Error>> {
        let out = File::create(out.as_ref())?;
        Ok(SpiceBackend {
            generated: HashMap::new(),
            out,
            mos_name_fn: sky130_mos_name,
            top_name: None,
        })
    }

    pub fn with_file(out: File) -> Result<SpiceBackend<File>, Box<dyn std::error::Error>> {
        Ok(SpiceBackend {
            generated: HashMap::new(),
            out,
            mos_name_fn: sky130_mos_name,
            top_name: None,
        })
    }
}

impl<T> SpiceBackend<T>
where
    T: std::io::Write,
{
    pub fn new(out: T) -> Self {
        Self {
            generated: HashMap::new(),
            out,
            mos_name_fn: sky130_mos_name,
            top_name: None,
        }
    }

    pub fn set_top_name(&mut self, name: String) -> &mut Self {
        self.top_name = Some(name);
        self
    }

    fn is_generated(&self, module: &dyn Module) -> bool {
        let tid = (&*module).type_id();
        let mod_name = module.name();
        if let Some(hset) = self.generated.get(&tid) {
            hset.contains(&mod_name)
        } else {
            false
        }
    }

    fn mark_generated(&mut self, module: &dyn Module) {
        let tid = (&*module).type_id();
        let mod_name = module.name();
        if let Some(hset) = self.generated.get_mut(&tid) {
            hset.insert(mod_name);
        } else {
            let mut hset = HashSet::new();
            hset.insert(mod_name);
            self.generated.insert(tid, hset);
        }
    }

    pub fn netlist(&mut self, tree: &ContextTree) -> Result<(), Box<dyn std::error::Error>> {
        self.netlist_inner(tree, true)
    }

    pub fn netlist_inner(
        &mut self,
        tree: &ContextTree,
        top: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let module = &*tree.module;
        if self.is_generated(module) {
            return Ok(());
        }
        self.mark_generated(module);

        let name = if top {
            self.top_name.clone().unwrap_or_else(|| module.name())
        } else {
            module.name()
        };

        write!(self.out, ".subckt {}", name)?;
        self.netlist_module_ports(tree)?;
        writeln!(self.out)?;

        for (i, m) in tree.ctx.modules.iter().enumerate() {
            write!(self.out, "XM{}", i)?;
            let inst_ports = m.get_ports();
            for port in inst_ports {
                for node in port.signal.nodes() {
                    write!(self.out, " {}", tree.ctx.name(node))?;
                }
            }
            writeln!(self.out, " {}", m.name())?;
        }

        for (i, r) in tree.ctx.resistors.iter().enumerate() {
            writeln!(
                self.out,
                "R{} {} {} {}p",
                i,
                tree.ctx.name(r.a()),
                tree.ctx.name(r.b()),
                r.value().picoohms()
            )?;
        }
        for (i, m) in tree.ctx.mosfets.iter().enumerate() {
            let descriptor = (self.mos_name_fn)(m.flavor, m.intent.clone());
            writeln!(
                self.out,
                "XFET{} {} {} {} {} {} w={}m l={}m",
                i,
                tree.ctx.name(m.d),
                tree.ctx.name(m.g),
                tree.ctx.name(m.s),
                tree.ctx.name(m.b),
                descriptor,
                m.width_nm,
                m.length_nm,
            )?;
        }

        writeln!(self.out, ".ends")?;

        for m in tree.children.iter() {
            self.netlist_inner(m, false)?;
        }

        Ok(())
    }

    fn netlist_module_ports(
        &mut self,
        tree: &ContextTree,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for port in &tree.ctx.ports {
            for node in port.signal.nodes() {
                write!(self.out, " {}", tree.ctx.name(node))?;
            }
        }
        Ok(())
    }

    pub fn output(self) -> T {
        self.out
    }
}
