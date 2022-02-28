use crate::Node;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum MosType {
    Nmos,
    Pmos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MosIntent {
    /// Ultra low threshold voltage
    Ulvt,
    /// Low threshold voltage
    Lvt,
    /// Standard threshold voltage
    Svt,
    /// High threshold voltage
    Hvt,
    /// Ultra-high threshold voltage
    Uhvt,
    /// A custom transistor flavor; not directly supported by `micro_hdl`.
    Custom(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Mosfet {
    // length, width, n/p, lvt/hvt/etc, terminals (optional sub)
    pub width_nm: i64,
    pub length_nm: i64,
    pub mos_type: MosType,
    pub intent: MosIntent,

    // terminals
    pub d: Node,
    pub g: Node,
    pub s: Node,
    pub b: Node,
    /// Optional substrate terminal for DNW devices
    pub substrate: Option<Node>,
}
