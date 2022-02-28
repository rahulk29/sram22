use crate::context::ContextTree;
use crate::node::Node;
use crate::{Context, Module, ModuleConfig, Signal};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

pub struct SpiceBackend<T>
where
    T: std::io::Write,
{
    generated: HashMap<TypeId, HashSet<String>>,
    out: T,
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
        })
    }

    pub fn with_file(out: File) -> Result<SpiceBackend<File>, Box<dyn std::error::Error>> {
        Ok(SpiceBackend {
            generated: HashMap::new(),
            out,
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
        }
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
        let module = &*tree.module;
        if self.is_generated(module) {
            return Ok(());
        }
        self.mark_generated(module);

        write!(self.out, ".subckt {}", module.name())?;
        self.netlist_module_ports(&*module)?;
        writeln!(self.out)?;

        for (i, m) in tree.ctx.modules.iter().enumerate() {
            write!(self.out, "X{}", i)?;
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

        writeln!(self.out, ".ends")?;

        for m in tree.children.iter() {
            self.netlist(m)?;
        }

        Ok(())
    }

    fn netlist_module_ports(
        &mut self,
        module: &dyn Module,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for port in module.get_ports() {
            match port.signal {
                Signal::Wire(_) => {
                    write!(self.out, " {}", &port.name)?;
                }
                Signal::Bus(bus) => {
                    for (i, _) in bus.iter().enumerate() {
                        write!(self.out, " {}_{}", &port.name, i)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn output(self) -> T {
        self.out
    }
}
