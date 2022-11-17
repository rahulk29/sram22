use super::utils::{is_logical_high, is_logical_low};

#[derive(Clone)]
pub struct Waveform {
    /// List of `(t, x)` pairs.
    values: Vec<(f64, f64)>,
}

impl Waveform {
    #[inline]
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn with_initial_value(x: f64) -> Self {
        Self {
            values: vec![(0f64, x)],
        }
    }

    pub fn push(&mut self, t: f64, x: f64) {
        self.values.push((t, x));
    }

    pub fn last_t(&self) -> Option<f64> {
        self.values.last().map(|v| v.0)
    }

    pub fn last_x(&self) -> Option<f64> {
        self.values.last().map(|v| v.1)
    }

    pub fn last(&self) -> Option<(f64, f64)> {
        self.values.last().copied()
    }

    pub fn push_high(&mut self, until: f64, vdd: f64, tr: f64) {
        if is_logical_low(self.last_x().unwrap_or(vdd), vdd) {
            self.push(tr, vdd);
        }
        self.push(until, vdd);
    }

    pub fn push_low(&mut self, until: f64, vdd: f64, tf: f64) {
        if is_logical_high(self.last_x().unwrap_or(0f64), vdd) {
            self.push(tf, 0f64);
        }
        self.push(until, 0f64);
    }
}

impl Default for Waveform {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
