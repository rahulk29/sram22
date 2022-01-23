#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Node {
    pub(crate) id: u64,
    pub(crate) priority: i64,
}

impl Node {
    pub(crate) fn with_id(id: u64) -> Self {
        Self { id, priority: 0 }
    }
    pub(crate) fn gt_priority(&self, other: Node) -> bool {
        if self.priority == other.priority {
            return self.id < other.id;
        }
        self.priority > other.priority
    }

    pub fn test() -> Self {
        Node {
            id: 1,
            priority: -1000,
        }
    }
}
