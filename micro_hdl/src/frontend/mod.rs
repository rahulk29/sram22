use std::sync::Arc;

use crate::context::{Context, ContextTree};
use crate::{AbstractModule, Module, Signal};

pub fn parse<M>(m: M) -> ContextTree
where
    M: AbstractModule + std::any::Any,
{
    parse_boxed(Arc::new(m))
}

pub(crate) fn parse_boxed(m: Arc<dyn AbstractModule>) -> ContextTree {
    let mut c = Context::new();
    let top = m.generate(&mut c);

    parse_module(top)
}

fn parse_module(m: Arc<dyn Module>) -> ContextTree {
    let mut c = Context::new();
    let iports = m.generate(&mut c);
    let aports = m.get_ports();

    for (i, a) in iports.into_iter().zip(aports.into_iter()) {
        assert_eq!(i.width(), a.signal.width());
        c.make_port(a.name, a.pin_type, i);
    }

    let children = c
        .modules
        .iter()
        .map(|m| parse_module(Arc::clone(m)))
        .collect();
    ContextTree::new(c, m, children)
}
