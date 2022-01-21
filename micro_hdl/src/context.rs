use crate::signal::Signal;
use crate::Module;
use std::collections::HashMap;

pub struct Context {
    pub(crate) net_id: u64,
    pub(crate) modules: Vec<Box<dyn Module>>,
    pub(crate) net_names: HashMap<u64, String>,
    remap: HashMap<u64, u64>,
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn signal(&mut self) -> Signal {
        self.net_id += 1;
        self.net_names
            .insert(self.net_id, format!("net{}", self.net_id));
        Signal {
            id: self.net_id,
            priority: -1,
        }
    }

    pub fn connect(&mut self, a: Signal, b: Signal) {
        if a.gt_priority(b) {
            self.remap.insert(b.id, a.id);
        } else {
            self.remap.insert(a.id, b.id);
        }
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

    pub(crate) fn register_named_net(&mut self, name: &str) -> Signal {
        self.net_id += 1;
        self.net_names.insert(self.net_id, name.to_string());
        Signal {
            id: self.net_id,
            priority: 1,
        }
    }

    pub(crate) fn name(&self, s: Signal) -> String {
        let mut id = s.id;
        while let Some(&tmp) = self.remap.get(&id) {
            id = tmp;
        }

        self.net_names.get(&id).unwrap().to_string()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            net_id: 0,
            modules: vec![],
            net_names: HashMap::new(),
            remap: HashMap::new(),
        }
    }
}
