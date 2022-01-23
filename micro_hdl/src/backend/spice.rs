use crate::node::Node;
use crate::{Context, Module, ModuleConfig, Signal};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

pub struct SpiceBackend<T>
where
    T: std::io::Write,
{
    ts_id: u64,
    top_names: HashMap<u64, String>,
    generated: HashMap<TypeId, HashSet<String>>,
    out: T,
}

impl<T> SpiceBackend<T>
where
    T: std::io::Write,
{
    pub fn new(out: T) -> Self {
        Self {
            ts_id: 0,
            top_names: HashMap::new(),
            generated: HashMap::new(),
            out,
        }
    }

    pub fn top_level_signal(&mut self) -> Node {
        self.ts_id += 1;
        self.top_names
            .insert(self.ts_id, format!("top{}", self.ts_id));
        Node {
            id: self.ts_id,
            priority: 2,
        }
    }

    pub fn top_level_bus(&mut self, width: usize) -> Vec<Node> {
        (0..width).map(|_| self.top_level_signal()).collect()
    }

    pub fn netlist<M>(&mut self, top: M)
    where
        M: Module,
    {
        self.netlist_boxed(Box::new(top)).unwrap();
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

    #[allow(clippy::needless_collect)]
    fn netlist_module_internal(
        &mut self,
        module: Box<dyn Module>,
    ) -> Result<Context, Box<dyn std::error::Error>> {
        let mut ctx = Context::new();
        let port_signals = module
            .get_ports()
            .into_iter()
            .map(|port| {
                match port.signal.clone() {
                    Signal::Wire(_) => {
                        ctx.register_named_net(&port.name);
                    }
                    Signal::Bus(nodes) => {
                        for i in 0..nodes.len() {
                            ctx.register_named_net(&format!("{}_{}", &port.name, i));
                        }
                    }
                }
                port.signal
            })
            .collect::<Vec<_>>();

        let instance_pins = module.generate(&mut ctx);

        for (port_sig, inst_sig) in port_signals.into_iter().zip(instance_pins) {
            assert_eq!(port_sig.width(), inst_sig.width());
            for (a, b) in port_sig.nodes().zip(inst_sig.nodes()) {
                ctx.connect(a, b);
            }
        }

        for (i, m) in ctx.modules.iter().enumerate() {
            write!(self.out, "X{}", i)?;
            let inst_ports = m.get_ports();
            for port in inst_ports {
                for node in port.signal.nodes() {
                    write!(self.out, " {}", ctx.name(node))?;
                }
            }
            writeln!(self.out, " {}", m.name())?;
        }

        Ok(ctx)
    }

    fn netlist_boxed(&mut self, module: Box<dyn Module>) -> Result<(), Box<dyn std::error::Error>> {
        let tid = (&*module).type_id();
        // if we've already generated this module, continue
        let mod_name = module.name();
        if let Some(hset) = self.generated.get_mut(&tid) {
            if hset.contains(&mod_name) {
                return Ok(());
            } else {
                hset.insert(mod_name);
            }
        } else {
            let mut hset = HashSet::new();
            hset.insert(mod_name);
            self.generated.insert(tid, hset);
        }

        write!(self.out, ".subckt {}", module.name())?;
        self.netlist_module_ports(&*module)?;
        writeln!(self.out)?;

        match module.config() {
            ModuleConfig::Raw => {
                writeln!(self.out, "{}", module.spice())?;
                writeln!(self.out, ".ends")?;
            }
            ModuleConfig::Generate => {
                let ctx = self.netlist_module_internal(module)?;

                writeln!(self.out, ".ends")?;
                for m in ctx.modules {
                    self.netlist_boxed(m)?;
                }
            }
        }

        Ok(())
    }

    pub fn output(self) -> T {
        self.out
    }
}
