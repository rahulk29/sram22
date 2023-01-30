use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GateType {
    INV,
    NAND2,
    NAND3,
}

#[derive(Debug)]
pub struct GateInstance {
    gate_type: GateType,
    input_cap: f64,
    delay: f64,
    load_cap: f64,
}

#[derive(Copy, Clone, Debug)]
enum Element {
    Gate(GateType),
    Branch(f64),
}

pub fn logical_effort(gate: GateType, j: f64) -> f64 {
    match gate {
        GateType::INV => 1.0,
        GateType::NAND2 => (j + 2.0) / (j + 1.0),
        GateType::NAND3 => (j + 3.0) / (j + 1.0),
    }
}

pub fn parasitic_delay(gate: GateType) -> f64 {
    match gate {
        GateType::INV => 1.0,
        GateType::NAND2 => 2.0,
        GateType::NAND3 => 3.0,
    }
}

pub fn delay(gate: GateType, fanout: f64, j: f64, gamma: f64) -> f64 {
    gamma * parasitic_delay(gate) + logical_effort(gate, j) * fanout
}

impl Display for GateType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GateType::INV => write!(f, "INV"),
            GateType::NAND2 => write!(f, "NAND2"),
            GateType::NAND3 => write!(f, "NAND3"),
        }
    }
}

#[derive(Debug)]
pub struct FanoutAnalyzer {
    elements: Vec<Element>,
    num_stages: u32,
    j: f64,
    gamma: f64,
}

#[derive(Debug)]
pub struct FanoutResult {
    gates: Vec<GateInstance>,
    fanout: f64,
}

impl Default for FanoutAnalyzer {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            num_stages: 0,
            j: 1.0f64,
            gamma: 1.0f64,
        }
    }
}

impl<'a> FanoutResult {
    pub fn sizes(&'a self) -> impl Iterator<Item = f64> + 'a {
        self.gates.iter().map(|g| g.input_cap)
    }

    pub fn total_delay(&self) -> f64 {
        let mut delay = 0.0f64;
        for g in self.gates.iter() {
            delay += g.delay;
        }
        delay
    }
}

impl Display for FanoutResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "---- Fanout Result ----")?;
        for gate in self.gates.iter() {
            writeln!(
                f,
                "{}: {:.2}x (load cap: {:.2}, delay: {:.3})",
                gate.gate_type, gate.input_cap, gate.load_cap, gate.delay
            )?;
        }

        writeln!(f, "load capacitance: {:.2}", self.fanout)?;
        writeln!(f, "total delay: {:.3}", self.total_delay())?;
        writeln!(f, "-----------------------")
    }
}

impl FanoutAnalyzer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_j(mut self, j: f64) -> Self {
        self.j = j;
        self
    }

    pub fn with_gamma(mut self, gamma: f64) -> Self {
        self.gamma = gamma;
        self
    }

    pub fn add_gate(&mut self, gate: GateType) {
        self.num_stages += 1;
        self.elements.push(Element::Gate(gate));
    }

    pub fn add_branch(&mut self, branching_factor: f64) {
        self.elements.push(Element::Branch(branching_factor));
    }

    pub fn size(self, fanout: f64) -> FanoutResult {
        let mut b = 1.0f64;
        let mut g = 1.0f64;

        for element in self.elements.iter() {
            match *element {
                Element::Gate(gt) => {
                    g *= logical_effort(gt, self.j);
                }
                Element::Branch(bi) => {
                    b *= bi;
                }
            }
        }

        let path_effort = b * g * fanout;
        let stage_effort = path_effort.powf(1.0f64 / (self.num_stages as f64));

        let mut entries = Vec::new();
        let mut prev_size = fanout;
        let mut prev_b = 1.0f64;

        for element in self.elements.iter().rev() {
            match *element {
                Element::Gate(gt) => {
                    let load_cap = prev_size * prev_b;
                    prev_size = logical_effort(gt, self.j) * load_cap / stage_effort;
                    let fanout = load_cap / prev_size;
                    let delay = delay(gt, fanout, self.j, self.gamma);
                    entries.push(GateInstance {
                        gate_type: gt,
                        input_cap: prev_size,
                        delay,
                        load_cap,
                    });
                    prev_b = 1.0f64;
                }
                Element::Branch(bf) => {
                    prev_b = bf;
                }
            }
        }

        entries.reverse();

        FanoutResult {
            gates: entries,
            fanout,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{FanoutAnalyzer, GateType};

    #[test]
    fn test_inv_chain_2() {
        let mut f = FanoutAnalyzer::new();
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        let result = f.size(64f64);
        let sizes = result.sizes().collect::<Vec<f64>>();
        assert_approx_eq(sizes, vec![1.0, 8.0]);
    }

    #[test]
    fn test_inv_chain_3() {
        let mut f = FanoutAnalyzer::new();
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        let result = f.size(64f64);
        let sizes = result.sizes().collect::<Vec<f64>>();
        assert_approx_eq(sizes, vec![1.0, 4.0, 16.0]);
    }

    #[test]
    fn test_inv_chain_4() {
        let mut f = FanoutAnalyzer::new();
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        let result = f.size(64f64);
        let sizes = result.sizes().collect::<Vec<f64>>();
        assert_approx_eq(sizes, vec![1.0, 2.8284271247, 8.0, 22.627416998]);
    }

    #[test]
    fn test_inv_nand3_nand2() {
        let mut f = FanoutAnalyzer::new();
        f.add_gate(GateType::INV);
        f.add_branch(4.0);
        f.add_gate(GateType::NAND3);
        f.add_gate(GateType::NAND2);
        let result = f.size(18.0);
        assert_approx_eq_one(result.total_delay(), 24.0);

        let sizes = result.sizes().collect::<Vec<f64>>();
        assert_approx_eq(sizes, vec![1.0, 1.5, 4.5]);
    }

    #[test]
    fn test_inv_nand3_nand2_buf() {
        let mut f = FanoutAnalyzer::new();
        f.add_gate(GateType::INV);
        f.add_branch(4.0);
        f.add_gate(GateType::NAND3);
        f.add_gate(GateType::NAND2);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        let result = f.size(18.0);
        assert_approx_eq_one(result.total_delay(), 22.65078025);
    }

    #[test]
    fn test_inv_nand3_nand2_buf_buf() {
        let mut f = FanoutAnalyzer::new();
        f.add_gate(GateType::INV);
        f.add_branch(4.0);
        f.add_gate(GateType::NAND3);
        f.add_gate(GateType::NAND2);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        let result = f.size(18.0);
        assert_approx_eq_one(result.total_delay(), 25.0866019888);
    }

    #[test]
    fn test_inv_chain_4_gamma() {
        let mut f = FanoutAnalyzer::new().with_gamma(1.3);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        f.add_gate(GateType::INV);
        let result = f.size(700.0);
        let sizes = result.sizes().collect::<Vec<f64>>();
        assert_approx_eq(sizes, vec![1.0, 5.1436867236, 26.4575131106, 136.08915893]);
        assert_approx_eq_one(result.total_delay(), 25.774746894441606);
    }

    fn assert_approx_eq_one(x: f64, y: f64) {
        assert!(
            (x - y).abs() < 0.00000001 * x,
            "difference between {x} and {y} too large",
        );
    }

    fn assert_approx_eq(v1: Vec<f64>, v2: Vec<f64>) {
        assert_eq!(
            v1.len(),
            v2.len(),
            "array lengths not equal: {} != {}",
            v1.len(),
            v2.len()
        );
        for (i, x) in v1.iter().enumerate() {
            let y = v2.get(i).unwrap();
            assert_approx_eq_one(*x, *y);
        }
    }
}
