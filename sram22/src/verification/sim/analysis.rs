use std::collections::HashMap;

pub struct Analysis {
    pub(crate) mode: Mode,
    pub(crate) save: Vec<String>,
}

pub struct AnalysisData {
    pub data: HashMap<String, SpiceData>,
}

impl Default for AnalysisData {
    fn default() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

pub enum SpiceData {
    Real(Vec<f64>),
    Complex(Vec<f64>, Vec<f64>),
}

pub enum Mode {
    Tran(TransientAnalysis),
    Ac(AcAnalysis),
    Dc(DcAnalysis),
    Op,
}

pub struct DcAnalysis {
    source: String,
    start: f64,
    stop: f64,
    incr: f64,
}

pub struct TransientAnalysis {
    tstep: f64,
    tstop: f64,
    tstart: f64,
    uic: bool,
}

pub enum SweepMode {
    Dec,
    Oct,
    Lin,
}

pub struct AcAnalysis {
    mode: SweepMode,
    num: u64,
    fstart: f64,
    fstop: f64,
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
