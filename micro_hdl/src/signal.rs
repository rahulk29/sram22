#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Signal {
    pub(crate) id: u64,
    pub(crate) priority: i64,
}

impl Signal {
    pub(crate) fn with_id(id: u64) -> Self {
        Self { id, priority: 0 }
    }
    pub(crate) fn gt_priority(&self, other: Signal) -> bool {
        if self.priority > other.priority {
            true
        } else if self.priority < other.priority {
            false
        } else {
            self.id < other.id
        }
    }
}
