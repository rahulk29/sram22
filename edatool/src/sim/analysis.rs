use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Analysis {
    pub(crate) mode: Mode,
    pub(crate) save: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct AnalysisData {
    pub analysis: Analysis,
    pub data: HashMap<String, SpiceData>,
}

impl Analysis {
    pub fn with_mode(mode: Mode) -> Self {
        Self { mode, save: vec![] }
    }

    pub fn save(&mut self, v: &str) -> &mut Self {
        self.save.push(v.to_string());
        self
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum SpiceData {
    Real(Vec<f64>),
    Complex(Vec<f64>, Vec<f64>),
}

impl SpiceData {
    pub fn real(self) -> Vec<f64> {
        match self {
            Self::Real(x) => x,
            _ => panic!("called real on a complex SpiceData object"),
        }
    }

    pub fn complex(self) -> (Vec<f64>, Vec<f64>) {
        match self {
            Self::Complex(a, b) => (a, b),
            _ => panic!("called complex on a real SpiceData object"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum Mode {
    Tran(TransientAnalysis),
    Ac(AcAnalysis),
    Dc(DcAnalysis),
    Op,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, PartialEq)]
pub struct DcAnalysis {
    pub(crate) source: String,
    pub(crate) start: f64,
    pub(crate) stop: f64,
    pub(crate) incr: f64,
}

#[derive(Debug, Deserialize, Serialize, Default, Copy, Clone, PartialEq)]
pub struct TransientAnalysis {
    pub(crate) tstep: f64,
    pub(crate) tstop: f64,
    pub(crate) tstart: f64,
    pub(crate) uic: bool,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, Eq, PartialEq)]
pub enum SweepMode {
    Dec,
    Oct,
    Lin,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialEq)]
pub struct AcAnalysis {
    pub(crate) mode: SweepMode,
    pub(crate) num: u64,
    pub(crate) fstart: f64,
    pub(crate) fstop: f64,
}

impl Display for SweepMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SweepMode::Dec => write!(f, "dec"),
            SweepMode::Oct => write!(f, "oct"),
            SweepMode::Lin => write!(f, "lin"),
        }
    }
}

impl DcAnalysis {
    #[inline]
    pub fn new(source: String, start: f64, stop: f64, incr: f64) -> Self {
        Self {
            source,
            start,
            stop,
            incr,
        }
    }
}

impl AcAnalysis {
    #[inline]
    pub fn new(fstart: f64, fstop: f64) -> Self {
        Self {
            mode: SweepMode::Dec,
            num: 20,
            fstart,
            fstop,
        }
    }

    #[inline]
    pub fn num(mut self, num: u64) -> Self {
        self.num = num;
        self
    }

    #[inline]
    pub fn mode(mut self, mode: SweepMode) -> Self {
        self.mode = mode;
        self
    }
}

impl TransientAnalysis {
    #[inline]
    pub fn new(tstop: f64) -> Self {
        Self {
            tstep: tstop / 100f64,
            tstop,
            tstart: 0f64,
            uic: false,
        }
    }

    #[inline]
    pub fn tstep(mut self, tstep: f64) -> Self {
        self.tstep = tstep;
        self
    }

    #[inline]
    pub fn tstop(mut self, tstop: f64) -> Self {
        self.tstop = tstop;
        self
    }

    #[inline]
    pub fn tstart(mut self, tstart: f64) -> Self {
        self.tstart = tstart;
        self
    }

    #[inline]
    pub fn uic(mut self, uic: bool) -> Self {
        self.uic = uic;
        self
    }
}
