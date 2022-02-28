use crate::node::Node;
use crate::primitive::mos::Mosfet;
use crate::primitive::resistor::Resistor;
use crate::Module;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct Context {
    pub(crate) net_id: u64,
    pub(crate) modules: Vec<Arc<dyn Module>>,
    pub(crate) net_names: HashMap<u64, String>,
    remap: HashMap<u64, u64>,

    // primitives
    pub(crate) resistors: Vec<Resistor>,
    pub(crate) mosfets: Vec<Mosfet>,
}

pub struct ContextTree {
    pub ctx: Context,
    pub module: Arc<dyn Module>,
    pub children: Vec<ContextTree>,
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn node_with_priority(&mut self, priority: i64) -> Node {
        self.net_id += 1;
        self.net_names
            .insert(self.net_id, format!("net{}", self.net_id));
        Node {
            id: self.net_id,
            priority,
        }
    }

    pub fn node(&mut self) -> Node {
        self.node_with_priority(-1)
    }

    pub fn bus(&mut self, width: usize) -> Vec<Node> {
        (0..width).map(|_| self.node()).collect()
    }

    pub fn connect(&mut self, a: Node, b: Node) {
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
        self.add_boxed(Arc::new(module));
    }

    fn add_boxed(&mut self, module: Arc<dyn Module>) {
        self.modules.push(module);
    }

    pub fn add_resistor(&mut self, resistor: Resistor) {
        self.resistors.push(resistor);
    }

    pub fn add_mosfet(&mut self, mosfet: Mosfet) {
        self.mosfets.push(mosfet);
    }

    pub(crate) fn register_named_net(&mut self, name: &str) -> Node {
        self.net_id += 1;
        self.net_names.insert(self.net_id, name.to_string());
        Node {
            id: self.net_id,
            priority: 1,
        }
    }

    fn get_root(&self, s: Node) -> u64 {
        let mut id = s.id;
        while let Some(&tmp) = self.remap.get(&id) {
            id = tmp;
        }
        id
    }

    pub(crate) fn name(&self, s: Node) -> String {
        self.net_names.get(&self.get_root(s)).unwrap().to_string()
    }

    pub(crate) fn rename_net(&mut self, node: Node, name: &str) {
        self.net_names.insert(self.get_root(node), name.to_string());
    }
}

impl ContextTree {
    pub fn from_module(ctx: Context, module: Arc<dyn Module>) -> Self {
        Self {
            ctx,
            module,
            children: vec![],
        }
    }

    pub fn new(ctx: Context, module: Arc<dyn Module>, children: Vec<ContextTree>) -> Self {
        Self {
            ctx,
            module,
            children,
        }
    }
}
