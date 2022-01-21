use std::collections::{HashMap, HashSet};

pub mod primitive;

pub struct Context {
    net_id: u64,
    connected: Vec<(u64, u64)>,
    modules: Vec<Box<dyn Module>>,
    net_names: HashMap<u64, String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Signal {
    id: u64,
}

pub trait Module: ModuleInstance + std::any::Any {}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn signal(&mut self) -> Signal {
        self.net_id += 1;
        self.net_names
            .insert(self.net_id, format!("net{}", self.net_id));
        Signal { id: self.net_id }
    }

    pub fn connect(&mut self, a: Signal, b: Signal) {
        self.connected.push((a.id, b.id));
    }

    pub fn add<T>(&mut self, module: T)
    where
        T: Module,
    {
        self.add_boxed(Box::new(module));
    }

    fn add_boxed(&mut self, module: Box<dyn Module>) {
        self.modules.push(module);
    }

    fn register_named_net(&mut self, s: Signal, name: &str) {
        self.net_id += 1;
        self.net_names.insert(self.net_id, name.to_string());
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            net_id: 0,
            connected: vec![],
            modules: vec![],
            net_names: HashMap::new(),
        }
    }
}

pub enum PinType {
    Input,
    Output,
    InOut,
}

pub struct ModulePin {
    pub name: String,
    pub pin_type: PinType,
}

pub struct InstancePin {
    pub signal: Signal,
}

pub trait ModuleInstance {
    fn generate(&self, c: &mut Context) -> Vec<InstancePin>;
    fn params(&self) -> u64;
    fn name(&self) -> String;
    fn get_module_pins(&self) -> Vec<ModulePin>;
    fn get_instance_pins(&self) -> Vec<InstancePin>;
    fn config(&self) -> ModuleConfig;
}

pub struct SpiceBackend<T>
where
    T: std::io::Write,
{
    ts_id: u64,
    top_signals: Vec<Signal>,
    top_names: HashMap<u64, String>,
    generated: HashSet<String>,
    out: T,
}

impl<T> SpiceBackend<T>
where
    T: std::io::Write,
{
    pub fn new(out: T) -> Self {
        Self {
            ts_id: 0,
            top_signals: vec![],
            top_names: HashMap::new(),
            generated: HashSet::new(),
            out,
        }
    }

    pub fn top_level_signal(&mut self) -> Signal {
        self.ts_id += 1;
        self.top_names
            .insert(self.ts_id, format!("top{}", self.ts_id));
        Signal { id: self.ts_id }
    }

    pub fn netlist<M>(&mut self, top: M)
    where
        M: Module,
    {
        self.netlist_boxed(Box::new(top)).unwrap();
    }

    fn netlist_boxed(&mut self, top: Box<dyn Module>) -> Result<(), Box<dyn std::error::Error>> {
        write!(self.out, ".subckt {}", top.name())?;
        for pin in top.get_module_pins() {
            write!(self.out, " {}", pin.name)?;
        }
        writeln!(self.out, "")?;
        let mut ctx = Context::new();
        // TODO: need to rename pins
        let _ = top.generate(&mut ctx);

        let mut i = 0;
        for m in ctx.modules.iter() {
            write!(self.out, "X{}", i)?;
            let ipins = m.get_instance_pins();
            for pin in ipins {
                write!(self.out, " {}", pin.signal.id)?;
            }
            writeln!(self.out, " {}", m.name())?;
            i += 1;
        }

        writeln!(self.out, ".ends")?;

        for m in ctx.modules {
            self.netlist_boxed(m)?;
        }

        Ok(())
    }

    pub fn output(self) -> T {
        self.out
    }
}

pub enum ModuleConfig {
    Raw,
    Generate,
}

trait RawModule {
    fn spice(&self) -> String;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
