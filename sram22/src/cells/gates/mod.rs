use std::fmt::Display;

pub mod inv;
pub mod nand;
pub mod nand3;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct GateSize {
    pub nwidth_nm: u64,
    pub nlength_nm: u64,
    pub pwidth_nm: u64,
    pub plength_nm: u64,
}

impl Display for GateSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "n{}x{}_p{}x{}",
            self.nwidth_nm, self.nlength_nm, self.pwidth_nm, self.plength_nm
        )
    }
}

impl GateSize {
    pub fn minimum() -> Self {
        Self {
            nwidth_nm: 420,
            nlength_nm: 150,
            pwidth_nm: 420,
            plength_nm: 150,
        }
    }
}
