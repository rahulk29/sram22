use sram22::analysis::fanout::{FanoutAnalyzer, GateType};

fn main() {
    let fanout = 128.0;
    let mut f = FanoutAnalyzer::new();
    f.add_gate(GateType::INV);
    f.add_gate(GateType::NAND2);
    f.add_gate(GateType::INV);
    f.add_gate(GateType::INV);
    let result = f.size(fanout);
    println!("{}\n", result);
}
