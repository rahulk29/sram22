use crate::signal::Signal;
use crate::{Context, Module, ModuleConfig};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

pub struct SpiceBackend<T>
where
    T: std::io::Write,
{
    ts_id: u64,
    top_signals: Vec<Signal>,
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
            top_signals: vec![],
            top_names: HashMap::new(),
            generated: HashMap::new(),
            out,
        }
    }

    pub fn top_level_signal(&mut self) -> Signal {
        self.ts_id += 1;
        self.top_names
            .insert(self.ts_id, format!("top{}", self.ts_id));
        Signal {
            id: self.ts_id,
            priority: 2,
        }
    }

    pub fn netlist<M>(&mut self, top: M)
    where
        M: Module,
    {
        self.netlist_boxed(Box::new(top)).unwrap();
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
        for pin in module.get_module_pins() {
            write!(self.out, " {}", pin.name)?;
        }
        writeln!(self.out)?;

        match module.config() {
            ModuleConfig::Raw => {
                writeln!(self.out, "{}", module.spice())?;
                writeln!(self.out, ".ends")?;
            }
            ModuleConfig::Generate => {
                let mut ctx = Context::new();
                let pin_signals = module
                    .get_module_pins()
                    .into_iter()
                    .map(|pin| ctx.register_named_net(&pin.name))
                    .collect::<Vec<_>>();
                // TODO: need to rename pins
                let instance_pins = module.generate(&mut ctx);

                for (pin_sig, pin) in pin_signals.into_iter().zip(instance_pins) {
                    ctx.connect(pin_sig, pin.signal);
                }

                for (i, m) in ctx.modules.iter().enumerate() {
                    write!(self.out, "X{}", i)?;
                    let ipins = m.get_instance_pins();
                    for pin in ipins {
                        write!(self.out, " {}", ctx.name(pin.signal))?;
                    }
                    writeln!(self.out, " {}", m.name())?;
                }

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
