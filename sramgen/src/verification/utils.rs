use anyhow::bail;

use super::bit_signal::BitSignal;
use super::waveform::Waveform;

use crate::Result;

pub const DIGITAL_REL_TOL: f64 = 0.025;

pub fn is_logical_low(x: f64, vdd: f64) -> bool {
    (x / vdd).abs() < DIGITAL_REL_TOL
}

pub fn is_logical_high(x: f64, vdd: f64) -> bool {
    ((vdd - x) / vdd).abs() < DIGITAL_REL_TOL
}

pub fn logical_eq(x: f64, y: f64, vdd: f64) -> bool {
    ((x - y) / vdd).abs() < DIGITAL_REL_TOL
}

pub fn to_bit(x: f64, vdd: f64) -> Result<bool> {
    if is_logical_low(x, vdd) {
        Ok(false)
    } else if is_logical_high(x, vdd) {
        Ok(true)
    } else {
        bail!("Value was not close enough to either VDD or ground: {}", x);
    }
}

pub fn push_bus(
    waveforms: &mut [Waveform],
    signal: &BitSignal,
    until: f64,
    vdd: f64,
    tr: f64,
    tf: f64,
) {
    assert_eq!(waveforms.len(), signal.width());
    for (i, bit) in signal.bits().enumerate() {
        if bit {
            waveforms[i].push_high(until, vdd, tr);
        } else {
            waveforms[i].push_low(until, vdd, tf);
        }
    }
}
