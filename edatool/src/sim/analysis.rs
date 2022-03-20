use std::{fmt::Display, io::Write, path::Path, fs::File};

use prost::Message;

use crate::protos::sim::{sim_vector::Values, SimVector, SweepMode, SimulationData};

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

impl SimulationData {
    // Write simulation data to any object implementing `Write`.
    pub fn save<T>(&self, dst: &mut T) -> std::io::Result<()> where T: Write {
        let b = self.encode_to_vec();
        dst.write_all(&b)?;
        dst.flush()
    }
    
    // Saves simulation data to a file.
    pub fn to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut f = File::create(path)?;
        self.save(&mut f)?;
        Ok(())
    }
}
