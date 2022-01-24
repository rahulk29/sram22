use crate::{Context, Module, ModuleConfig, ModuleInstance, Node, PinType, Port, Signal};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MosParams {
    pub width_nm: u64,
    pub length_nm: u64,
}

pub struct Nmos {
    pub params: MosParams,
    pub d: Node,
    pub g: Node,
    pub s: Node,
    pub b: Node,
}

pub struct Pmos {
    pub params: MosParams,
    pub d: Node,
    pub g: Node,
    pub s: Node,
    pub b: Node,
}

impl Module for Nmos {}

impl ModuleInstance for Nmos {
    fn generate(&self, _c: &mut Context) -> Vec<Signal> {
        panic!("cannot generate nmos");
    }

    fn spice(&self) -> String {
        format!(
            "X1 d g s b sky130_fd_pr__nfet01v8 w={}m l={}m",
            self.params.width_nm, self.params.length_nm
        )
    }

    fn name(&self) -> String {
        format!(
            "nfet01v8_w{}m_l{}m",
            self.params.width_nm, self.params.length_nm
        )
    }

    fn get_ports(&self) -> Vec<Port> {
        vec![
            Port {
                name: "d".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.d),
            },
            Port {
                name: "g".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.g),
            },
            Port {
                name: "s".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.s),
            },
            Port {
                name: "b".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.b),
            },
        ]
    }

    fn config(&self) -> ModuleConfig {
        ModuleConfig::Raw
    }
}

impl Module for Pmos {}

impl ModuleInstance for Pmos {
    fn generate(&self, _c: &mut Context) -> Vec<Signal> {
        panic!("cannot generate pmos");
    }

    fn spice(&self) -> String {
        format!(
            "X1 d g s b sky130_fd_pr__pfet01v8 w={}m l={}m",
            self.params.width_nm, self.params.length_nm
        )
    }

    fn name(&self) -> String {
        format!(
            "pfet01v8_w{}m_l{}m",
            self.params.width_nm, self.params.length_nm
        )
    }

    fn get_ports(&self) -> Vec<Port> {
        vec![
            Port {
                name: "d".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.d),
            },
            Port {
                name: "g".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.g),
            },
            Port {
                name: "s".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.s),
            },
            Port {
                name: "b".to_string(),
                pin_type: PinType::InOut,
                signal: Signal::Wire(self.b),
            },
        ]
    }

    fn config(&self) -> ModuleConfig {
        ModuleConfig::Raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Module, ModuleInstance, Node};

    #[test]
    fn nmos_implements_module_instance() {
        let _: Box<dyn ModuleInstance> = Box::new(Nmos {
            params: MosParams {
                width_nm: 1_000,
                length_nm: 150,
            },
            d: Node::test(),
            g: Node::test(),
            s: Node::test(),
            b: Node::test(),
        });
    }

    #[test]
    fn nmos_implements_module() {
        let _: Box<dyn Module> = Box::new(Nmos {
            params: MosParams {
                width_nm: 1_000,
                length_nm: 150,
            },
            d: Node::test(),
            g: Node::test(),
            s: Node::test(),
            b: Node::test(),
        });
    }

    #[test]
    fn pmos_implements_module_instance() {
        let _: Box<dyn ModuleInstance> = Box::new(Pmos {
            params: MosParams {
                width_nm: 1_000,
                length_nm: 150,
            },
            d: Node::test(),
            g: Node::test(),
            s: Node::test(),
            b: Node::test(),
        });
    }

    #[test]
    fn pmos_implements_module() {
        let _: Box<dyn Module> = Box::new(Pmos {
            params: MosParams {
                width_nm: 1_000,
                length_nm: 150,
            },
            d: Node::test(),
            g: Node::test(),
            s: Node::test(),
            b: Node::test(),
        });
    }
}
