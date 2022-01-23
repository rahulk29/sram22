use crate::node::Node;
use crate::Module;
use std::collections::HashMap;

#[derive(Default)]
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
        self.add_boxed(Box::new(module));
    }

    fn add_boxed(&mut self, module: Box<dyn Module>) {
        self.modules.push(module);
    }

    pub(crate) fn register_named_net(&mut self, name: &str) -> Node {
        self.net_id += 1;
        self.net_names.insert(self.net_id, name.to_string());
        Node {
            id: self.net_id,
            priority: 1,
        }
    }

    pub(crate) fn name(&self, s: Node) -> String {
        let mut id = s.id;
        while let Some(&tmp) = self.remap.get(&id) {
            id = tmp;
        }

        self.net_names.get(&id).unwrap().to_string()
    }
}
