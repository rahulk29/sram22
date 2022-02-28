use crate::Node;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Flavor {
    Nmos,
    Pmos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
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
#[must_use = "creating a Mosfet instance does nothing; you must add it to a Context"]
pub struct Mosfet {
    // length, width, n/p, lvt/hvt/etc, terminals (optional sub)
    pub(crate) width_nm: i64,
    pub(crate) length_nm: i64,
    pub(crate) flavor: Flavor,
    pub(crate) intent: Intent,

    // terminals
    pub(crate) d: Node,
    pub(crate) g: Node,
    pub(crate) s: Node,
    pub(crate) b: Node,
    /// Optional substrate terminal for DNW devices
    pub(crate) substrate: Option<Node>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[must_use = "creating a MosfetBuilder instance does nothing; you must use it to build a Mosfet, and add the Mosfet to a Context"]
pub struct MosfetBuilder {
    width_nm: Option<i64>,
    length_nm: Option<i64>,
    flavor: Flavor,
    intent: Option<Intent>,

    d: Option<Node>,
    g: Option<Node>,
    s: Option<Node>,
    b: Option<Node>,
    substrate: Option<Node>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MosfetParams {
    pub width_nm: i64,
    pub length_nm: i64,
    pub flavor: Flavor,
    pub intent: Intent,
}

impl Default for Intent {
    fn default() -> Self {
        Self::Svt
    }
}

impl Mosfet {
    pub fn nmos() -> MosfetBuilder {
        MosfetBuilder {
            width_nm: None,
            length_nm: None,
            flavor: Flavor::Nmos,
            intent: None,

            d: None,
            g: None,
            s: None,
            b: None,
            substrate: None,
        }
    }

    pub fn pmos() -> MosfetBuilder {
        MosfetBuilder {
            width_nm: None,
            length_nm: None,
            flavor: Flavor::Pmos,
            intent: None,

            d: None,
            g: None,
            s: None,
            b: None,
            substrate: None,
        }
    }

    pub fn with_params(params: MosfetParams) -> MosfetBuilder {
        MosfetBuilder {
            width_nm: Some(params.width_nm),
            length_nm: Some(params.length_nm),
            flavor: params.flavor,
            intent: Some(params.intent),

            d: None,
            g: None,
            s: None,
            b: None,
            substrate: None,
        }
    }
}

impl MosfetBuilder {
    pub fn width_nm(mut self, width_nm: i64) -> Self {
        self.width_nm = Some(width_nm);
        self
    }
    pub fn length_nm(mut self, length_nm: i64) -> Self {
        self.length_nm = Some(length_nm);
        self
    }
    pub fn intent(mut self, intent: Intent) -> Self {
        self.intent = Some(intent);
        self
    }
    pub fn d(mut self, node: Node) -> Self {
        self.d = Some(node);
        self
    }
    pub fn g(mut self, node: Node) -> Self {
        self.g = Some(node);
        self
    }
    pub fn s(mut self, node: Node) -> Self {
        self.s = Some(node);
        self
    }
    pub fn b(mut self, node: Node) -> Self {
        self.b = Some(node);
        self
    }
    pub fn substrate(mut self, node: Node) -> Self {
        self.substrate = Some(node);
        self
    }
    pub fn build(self) -> Mosfet {
        Mosfet {
            width_nm: self.width_nm.unwrap(),
            length_nm: self.length_nm.unwrap(),
            flavor: self.flavor,
            intent: self.intent.unwrap_or_default(),
            d: self.d.unwrap(),
            g: self.g.unwrap(),
            s: self.s.unwrap(),
            b: self.b.unwrap(),
            substrate: self.substrate,
        }
    }
}
