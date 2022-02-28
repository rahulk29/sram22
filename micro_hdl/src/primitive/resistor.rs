use std::fmt::Display;

use crate::Node;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resistance {
    picoohms: i128,
}

pub struct Resistor {
    value: Resistance,
    a: Node,
    b: Node,
}

pub struct ResistorBuilder {
    value: Option<Resistance>,
    a: Option<Node>,
    b: Option<Node>,
}

impl Resistance {
    #[inline]
    pub fn from_picoohms(value: i128) -> Self {
        Self { picoohms: value }
    }
    #[inline]
    pub fn from_nanoohms(value: i128) -> Self {
        Self {
            picoohms: value * 1_000,
        }
    }
    #[inline]
    pub fn from_microohms(value: i128) -> Self {
        Self {
            picoohms: value * 1_000_000,
        }
    }
    #[inline]
    pub fn from_milliohms(value: i128) -> Self {
        Self {
            picoohms: value * 1_000_000_000,
        }
    }
    #[inline]
    pub fn from_ohms(value: i128) -> Self {
        Self {
            picoohms: value * 1_000_000_000_000,
        }
    }
    #[inline]
    pub fn from_kiloohms(value: i128) -> Self {
        Self {
            picoohms: value * 1_000_000_000_000_000,
        }
    }
    #[inline]
    pub fn from_megaohms(value: i128) -> Self {
        Self {
            picoohms: value * 1_000_000_000_000_000_000,
        }
    }
    #[inline]
    pub fn from_gigaohms(value: i128) -> Self {
        Self {
            picoohms: value * 1_000_000_000_000_000_000_000,
        }
    }
    #[inline]
    pub fn picoohms(&self) -> i128 {
        self.picoohms
    }
}

impl Display for Resistance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}p\u{2_126}", self.picoohms)
    }
}

impl Resistor {
    pub fn instance() -> ResistorBuilder {
        ResistorBuilder {
            value: None,
            a: None,
            b: None,
        }
    }

    #[inline]
    pub fn value(&self) -> Resistance {
        self.value
    }
    #[inline]
    pub fn a(&self) -> Node {
        self.a
    }
    #[inline]
    pub fn b(&self) -> Node {
        self.b
    }
}

impl ResistorBuilder {
    pub fn value(mut self, value: Resistance) -> Self {
        self.value = Some(value);
        self
    }
    pub fn a(mut self, a: Node) -> Self {
        self.a = Some(a);
        self
    }
    pub fn b(mut self, b: Node) -> Self {
        self.b = Some(b);
        self
    }

    pub fn build(self) -> Resistor {
        Resistor {
            value: self.value.unwrap(),
            a: self.a.unwrap(),
            b: self.b.unwrap(),
        }
    }
}
