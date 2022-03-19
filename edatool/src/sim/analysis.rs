use std::fmt::Display;

use crate::protos::sim::{sim_vector::Values, SimVector, SweepMode};

impl Display for SweepMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SweepMode::Decade => write!(f, "dec"),
            SweepMode::Octave => write!(f, "oct"),
            SweepMode::Linear => write!(f, "lin"),
            _ => panic!("unknown sweep mode"),
        }
    }
}

impl SimVector {
    pub fn unwrap_real(self) -> Vec<f64> {
        match self.values {
            Some(Values::Real(v)) => v.v,
            _ => panic!("called unwrap_real on a SimVector that was empty or had complex values"),
        }
    }
    pub fn unwrap_complex(self) -> (Vec<f64>, Vec<f64>) {
        match self.values {
            Some(Values::Complex(v)) => (v.a, v.b),
            _ => panic!("called unwrap_complex on a SimVector that was empty or had real values"),
        }
    }
}
