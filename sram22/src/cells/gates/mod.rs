use std::fmt::Display;

use magic_vlsi::units::Distance;

use crate::config::TechConfig;

pub mod inv;
pub mod nand;
pub mod nand3;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct GateSize {
    pub nwidth_nm: i64,
    pub nlength_nm: i64,
    pub pwidth_nm: i64,
    pub plength_nm: i64,
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

pub(crate) fn finger_space(tc: &TechConfig) -> Distance {
    [
        2 * tc.space("gate", "licon") + tc.layer("li").width,
        tc.layer("poly").space,
    ]
    .into_iter()
    .max()
    .unwrap()
}

pub(crate) fn ndiff_edge_to_gate(tc: &TechConfig) -> Distance {
    [
        tc.layer("ndiff").extension("poly"),
        tc.space("gate", "licon") + tc.layer("licon").width + tc.layer("licon").enclosure("ndiff"),
    ]
    .into_iter()
    .max()
    .unwrap()
}

pub(crate) fn pdiff_edge_to_gate(tc: &TechConfig) -> Distance {
    [
        tc.layer("pdiff").extension("poly"),
        tc.space("gate", "licon") + tc.layer("licon").width + tc.layer("licon").enclosure("ndiff"),
    ]
    .into_iter()
    .max()
    .unwrap()
}
