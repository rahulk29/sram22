// Open-source Liberty (.lib) generator for SRAM macros.
// Produces the same structural format as Liberate MX CCS output.
// Numeric values are provided by a pluggable `TimingModel` impl.

use crate::blocks::sram::SramParams;
use anyhow::Result;
use std::path::PathBuf;

// ── INDEX AXES ────────────────────────────────────────────────────────────────

const SLEW_IDX: [f64; 7] = [0.002, 0.008, 0.03, 0.073, 0.138, 0.231, 0.351];
const LOAD_IDX: [f64; 7] = [0.007, 0.013, 0.033, 0.065, 0.13, 0.26, 0.52];
const PWR_IDX: [f64; 7] = [0.002, 0.004, 0.011, 0.022, 0.035, 0.087, 0.17];

// ── MODEL TYPES ───────────────────────────────────────────────────────────────

/// PVT corner for Liberty characterization.
#[derive(Clone)]
pub struct PvtCorner {
    pub name: String,
    pub process: f64,
    pub voltage: f64,
    pub temperature: f64,
}

impl PvtCorner {
    /// Standard corners used in sram22: tt/ss/ff with their PVT values.
    pub fn tt() -> Self {
        Self {
            name: "tt".into(),
            process: 1.0,
            voltage: 1.8,
            temperature: 25.0,
        }
    }
    pub fn ss() -> Self {
        Self {
            name: "ss".into(),
            process: 1.0,
            voltage: 1.6,
            temperature: 100.0,
        }
    }
    pub fn ff() -> Self {
        Self {
            name: "ff".into(),
            process: 1.0,
            voltage: 1.95,
            temperature: -40.0,
        }
    }

    /// e.g. "PVT_1P95V_-40C"
    fn oc_name(&self) -> String {
        let v = fmt_voltage_oc(self.voltage);
        let t = if self.temperature < 0.0 {
            format!("-{}C", (-self.temperature) as i64)
        } else {
            format!("{}C", self.temperature as i64)
        };
        format!("PVT_{}V_{}", v, t)
    }

    /// e.g. "ff_n40C_1v95"
    pub fn file_suffix(&self) -> String {
        let t = if self.temperature < 0.0 {
            format!("n{}C", (-self.temperature) as i64)
        } else {
            format!("{:03}C", self.temperature as i64)
        };
        let v = format!("{:.2}", self.voltage).replace('.', "v");
        format!("{}_{}_{}", self.name, t, v)
    }
}

/// Capacitance values for a pin (in pF).
pub struct CapValues {
    pub cap: f64,
    pub rise: f64,
    pub fall: f64,
    pub rise_lo: f64,
    pub fall_lo: f64,
}

/// CCS current waveform for one (slew, load) operating point.
pub struct CcsVector {
    pub reference_time: f64,
    pub slew: f64,
    pub load: f64,
    pub time_pts: Vec<f64>,
    pub currents: Vec<f64>,
}

/// Logical role of a pin in the SRAM interface.
#[derive(Clone, Copy)]
pub enum PinRole {
    Addr,
    Din,
    Ce,
    We,
    Rstb,
    Wmask,
    Clk,
}

// ── TimingModel TRAIT ─────────────────────────────────────────────────────────

/// Plug-in point for timing equations.
/// `[[f64;7];7]` tables: rows index slew, columns index load or clock-slew.
pub trait TimingModel: Send + Sync {
    fn area(&self, p: &SramParams) -> f64;

    fn hold_rise(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7];
    fn hold_fall(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7];
    fn setup_rise(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7];
    fn setup_fall(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7];

    fn cell_rise(&self, p: &SramParams) -> [[f64; 7]; 7];
    fn cell_fall(&self, p: &SramParams) -> [[f64; 7]; 7];
    fn rise_transition(&self, p: &SramParams) -> [[f64; 7]; 7];
    fn fall_transition(&self, p: &SramParams) -> [[f64; 7]; 7];

    fn minimum_period(&self, p: &SramParams) -> [f64; 7];
    fn min_pulse_width(&self, p: &SramParams) -> ([f64; 7], [f64; 7]);

    fn input_cap(&self, p: &SramParams, pin: PinRole) -> CapValues;
    fn output_max_cap(&self, p: &SramParams) -> f64;

    fn receiver_cap_1_rise(&self, p: &SramParams, pin: PinRole) -> [f64; 7];
    fn receiver_cap_2_rise(&self, p: &SramParams, pin: PinRole) -> [f64; 7];
    fn receiver_cap_1_fall(&self, p: &SramParams, pin: PinRole) -> [f64; 7];
    fn receiver_cap_2_fall(&self, p: &SramParams, pin: PinRole) -> [f64; 7];

    fn ccs_rise(&self, p: &SramParams) -> Vec<CcsVector>;
    fn ccs_fall(&self, p: &SramParams) -> Vec<CcsVector>;

    fn clk_power_rise(&self, p: &SramParams, when: &str) -> [f64; 7];
    fn clk_power_fall(&self, p: &SramParams, when: &str) -> [f64; 7];
}

/// Placeholder: returns zeros for all values. Produces structurally valid Liberty
/// output that can be syntax-checked without any characterized data.
pub struct PlaceholderModel;

impl TimingModel for PlaceholderModel {
    fn area(&self, _: &SramParams) -> f64 {
        0.0
    }
    fn hold_rise(&self, _: &SramParams, _: PinRole) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn hold_fall(&self, _: &SramParams, _: PinRole) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn setup_rise(&self, _: &SramParams, _: PinRole) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn setup_fall(&self, _: &SramParams, _: PinRole) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn cell_rise(&self, _: &SramParams) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn cell_fall(&self, _: &SramParams) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn rise_transition(&self, _: &SramParams) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn fall_transition(&self, _: &SramParams) -> [[f64; 7]; 7] {
        [[0.0; 7]; 7]
    }
    fn minimum_period(&self, _: &SramParams) -> [f64; 7] {
        [0.0; 7]
    }
    fn min_pulse_width(&self, _: &SramParams) -> ([f64; 7], [f64; 7]) {
        ([0.0; 7], [0.0; 7])
    }
    fn input_cap(&self, _: &SramParams, _: PinRole) -> CapValues {
        CapValues {
            cap: 0.0,
            rise: 0.0,
            fall: 0.0,
            rise_lo: 0.0,
            fall_lo: 0.0,
        }
    }
    fn output_max_cap(&self, _: &SramParams) -> f64 {
        0.52
    }
    fn receiver_cap_1_rise(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }
    fn receiver_cap_2_rise(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }
    fn receiver_cap_1_fall(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }
    fn receiver_cap_2_fall(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }
    fn ccs_rise(&self, _: &SramParams) -> Vec<CcsVector> {
        SLEW_IDX
            .iter()
            .flat_map(|&slew| {
                LOAD_IDX.iter().map(move |&load| CcsVector {
                    reference_time: 0.0,
                    slew,
                    load,
                    time_pts: vec![0.0, 1.0],
                    currents: vec![0.0, 0.0],
                })
            })
            .collect()
    }
    fn ccs_fall(&self, p: &SramParams) -> Vec<CcsVector> {
        self.ccs_rise(p)
    }
    fn clk_power_rise(&self, _: &SramParams, _: &str) -> [f64; 7] {
        [0.0; 7]
    }
    fn clk_power_fall(&self, _: &SramParams, _: &str) -> [f64; 7] {
        [0.0; 7]
    }
}

// ── LOOKUP MODEL ─────────────────────────────────────────────────────────────
//
// Equation forms (all linear-in-parameters so ordinary least squares suffices).
// Each basis returns [f64; 4]; trailing zeros are unused slots.

fn basis_lin4_sqrt_log(dw: f64) -> [f64; 4] {
    [1.0, dw, dw.sqrt(), dw.ln()]
}
fn basis_lin3_cbrt(dw: f64) -> [f64; 4] {
    [1.0, dw, dw.cbrt(), 0.0]
}
fn basis_lin3_sqrt(dw: f64) -> [f64; 4] {
    [1.0, dw, dw.sqrt(), 0.0]
}
fn basis_log2(dw: f64) -> [f64; 4] {
    [1.0, dw.ln(), 0.0, 0.0]
}
fn basis_lin2(dw: f64) -> [f64; 4] {
    [1.0, dw, 0.0, 0.0]
}
fn basis_log3(dw: f64) -> [f64; 4] {
    [1.0, dw.ln(), dw.ln().powi(2), 0.0]
}
fn basis_lin4_log_log2(dw: f64) -> [f64; 4] {
    let l = dw.ln();
    [1.0, dw, l, l.powi(2)]
}

/// One fitted entry: `eval(dw) = sum(params[k] * basis(dw)[k])`.
/// At characterized training points the original measured value is returned exactly;
/// the fitted polynomial is only used for interpolated (non-training) dw values.
#[derive(Clone)]
struct FitEntry {
    basis: fn(f64) -> [f64; 4],
    params: [f64; 4],
    training: Vec<(f64, f64)>, // (dw, val) pairs stored for exact-match lookup
}

impl FitEntry {
    fn eval(&self, dw: f64) -> f64 {
        // Return the exact characterized value when queried at a training point.
        if let Some(&(_, y)) = self.training.iter().find(|&&(x, _)| x == dw) {
            return y;
        }
        let x = (self.basis)(dw);
        x[0] * self.params[0]
            + x[1] * self.params[1]
            + x[2] * self.params[2]
            + x[3] * self.params[3]
    }

    /// OLS fit with two conservative lift passes:
    /// 1. Lift at training points so fitted ≥ actual at every training dw.
    /// 2. Lift at every integer dw in [min_dw, max_dw].  For each point the
    ///    reference is:
    ///    - Flat data (range/mean < 1%): global max of training values.
    ///      The data has no real dw trend; measurement noise can put any
    ///      intermediate value anywhere in the range, so the global max is
    ///      the only conservative bound.
    ///    - Trending data (range/mean ≥ 1%): piecewise log-linear interpolant
    ///      of training data — tight and conservative for smooth curves.
    fn fit(basis: fn(f64) -> [f64; 4], dws: &[f64], vals: &[f64]) -> Self {
        let mut p = ols4(basis, dws, vals);

        // Pass 1: training points
        let lift1 = dws
            .iter()
            .zip(vals)
            .map(|(&x, &y)| {
                let b = basis(x);
                y - (b[0] * p[0] + b[1] * p[1] + b[2] * p[2] + b[3] * p[3])
            })
            .fold(0.0_f64, f64::max);
        p[0] += lift1;

        // Pass 2: every integer dw between training extremes
        let val_max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let val_min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let val_mean = vals.iter().sum::<f64>() / vals.len() as f64;
        let is_flat = val_mean > 0.0 && (val_max - val_min) / val_mean < 0.02;
        let min_dw = dws[0] as u32;
        let max_dw = *dws.last().unwrap() as u32;
        let lift2 = (min_dw..=max_dw)
            .map(|dw_i| {
                let dw = dw_i as f64;
                let reference = if is_flat {
                    val_max
                } else {
                    piecewise_log_linear(dw, dws, vals)
                };
                let b = basis(dw);
                reference - (b[0] * p[0] + b[1] * p[1] + b[2] * p[2] + b[3] * p[3])
            })
            .fold(0.0_f64, f64::max);
        p[0] += lift2;

        let training = dws.iter().zip(vals).map(|(&x, &y)| (x, y)).collect();
        Self {
            basis,
            params: p,
            training,
        }
    }
}

fn piecewise_log_linear(dw: f64, dws: &[f64], vals: &[f64]) -> f64 {
    let i = dws.partition_point(|&x| x <= dw);
    if i == 0 {
        return vals[0];
    }
    if i >= dws.len() {
        return vals[dws.len() - 1];
    }
    let (x0, y0) = (dws[i - 1], vals[i - 1]);
    let (x1, y1) = (dws[i], vals[i]);
    let t = (dw.ln() - x0.ln()) / (x1.ln() - x0.ln());
    y0 + t * (y1 - y0)
}

fn ols4(basis: fn(f64) -> [f64; 4], dws: &[f64], vals: &[f64]) -> [f64; 4] {
    let mut xtx = [[0.0f64; 4]; 4];
    let mut xty = [0.0f64; 4];
    for (&dw, &y) in dws.iter().zip(vals) {
        let x = basis(dw);
        for i in 0..4 {
            xty[i] += x[i] * y;
            for j in 0..4 {
                xtx[i][j] += x[i] * x[j];
            }
        }
    }
    gauss4(xtx, xty)
}

fn gauss4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> [f64; 4] {
    for col in 0..4 {
        let pivot = (col..4)
            .max_by(|&i, &j| a[i][col].abs().partial_cmp(&a[j][col].abs()).unwrap())
            .unwrap();
        a.swap(col, pivot);
        b.swap(col, pivot);
        let d = a[col][col];
        if d.abs() < 1e-14 {
            continue;
        }
        for row in (col + 1)..4 {
            let f = a[row][col] / d;
            for k in col..4 {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = [0.0f64; 4];
    for i in (0..4).rev() {
        let s: f64 = ((i + 1)..4).map(|j| a[i][j] * x[j]).sum();
        x[i] = if a[i][i].abs() < 1e-14 {
            0.0
        } else {
            (b[i] - s) / a[i][i]
        };
    }
    x
}

fn eval_7x7(tbl: &[[FitEntry; 7]; 7], dw: f64) -> [[f64; 7]; 7] {
    std::array::from_fn(|i| std::array::from_fn(|j| tbl[i][j].eval(dw)))
}

/// Data-driven timing model loaded from a characterization JSON file.
///
/// Each timing table entry is fitted with a conservative upper-bound parametric
/// equation (linear in parameters, solved via OLS + intercept lift).
/// `din_hold_fall` uses a per-entry constant (= max measured value) because its
/// data has no dw trend (random ±1.5% noise around ~0.04 ns).
/// `min_period` uses piecewise log-linear interpolation with a monotone upper hull.
/// For the SS corner, Liberate MX fails to converge at dw=1024+ so those entries
/// are estimated using the SS/TT ratio (~1.828) computed from the valid dw range.
pub struct LookupModel {
    cell_rise: [[FitEntry; 7]; 7],
    cell_fall: [[FitEntry; 7]; 7],
    rise_transition: [[FitEntry; 7]; 7],
    fall_transition: [[FitEntry; 7]; 7],
    addr_hold_rise: [[FitEntry; 7]; 7],
    addr_hold_fall: [[FitEntry; 7]; 7],
    addr_setup_rise: [[FitEntry; 7]; 7],
    addr_setup_fall: [[FitEntry; 7]; 7],
    din_hold_rise: [[FitEntry; 7]; 7],
    din_hold_fall: [[f64; 7]; 7],
    din_setup_rise: [[FitEntry; 7]; 7],
    din_setup_fall: [[FitEntry; 7]; 7],
    ce_hold_rise: [[FitEntry; 7]; 7],
    ce_hold_fall: [[FitEntry; 7]; 7],
    ce_setup_rise: [[FitEntry; 7]; 7],
    ce_setup_fall: [[FitEntry; 7]; 7],
    we_hold_rise: [[FitEntry; 7]; 7],
    we_hold_fall: [[FitEntry; 7]; 7],
    we_setup_rise: [[FitEntry; 7]; 7],
    we_setup_fall: [[FitEntry; 7]; 7],
    rstb_hold_rise: [[FitEntry; 7]; 7],
    rstb_hold_fall: [[FitEntry; 7]; 7],
    rstb_setup_rise: [[FitEntry; 7]; 7],
    rstb_setup_fall: [[FitEntry; 7]; 7],
    mpw_rise: [FitEntry; 7],
    mpw_fall: [FitEntry; 7],
    min_period: Vec<(u32, [f64; 7])>,
    clk_cap: f64,
    addr_cap: f64,
    din_cap: f64,
    ce_cap: f64,
    we_cap: f64,
    rstb_cap: f64,
    wmask_cap: f64,
}

impl LookupModel {
    /// Build from embedded JSON bytes for one PVT corner ("tt", "ss", or "ff").
    pub fn from_json(json_bytes: &[u8], corner: &str) -> Result<Self> {
        let root: serde_json::Value = serde_json::from_slice(json_bytes)?;
        let cmap = root[corner]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("corner '{}' not found in timing JSON", corner))?;

        let mut dw_keys: Vec<u32> = cmap.keys().map(|k| k.parse().unwrap()).collect();
        dw_keys.sort_unstable();
        let dws: Vec<f64> = dw_keys.iter().map(|&k| k as f64).collect();

        let v2d = |dw: u32, key: &str, i: usize, j: usize| -> f64 {
            cmap[&dw.to_string()][key][i][j].as_f64().unwrap_or(0.0)
        };
        let v1d = |dw: u32, key: &str, j: usize| -> f64 {
            cmap[&dw.to_string()][key][j].as_f64().unwrap_or(0.0)
        };
        let v0d =
            |dw: u32, key: &str| -> f64 { cmap[&dw.to_string()][key].as_f64().unwrap_or(0.0) };

        let fit7 = |key: &str, basis: fn(f64) -> [f64; 4]| -> [[FitEntry; 7]; 7] {
            std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    let vals: Vec<f64> = dw_keys.iter().map(|&dw| v2d(dw, key, i, j)).collect();
                    FitEntry::fit(basis, &dws, &vals)
                })
            })
        };
        let max7 = |key: &str| -> [[f64; 7]; 7] {
            std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    dw_keys
                        .iter()
                        .map(|&dw| v2d(dw, key, i, j))
                        .fold(f64::NEG_INFINITY, f64::max)
                })
            })
        };
        let fit1 = |key: &str, basis: fn(f64) -> [f64; 4]| -> [FitEntry; 7] {
            std::array::from_fn(|j| {
                let vals: Vec<f64> = dw_keys.iter().map(|&dw| v1d(dw, key, j)).collect();
                FitEntry::fit(basis, &dws, &vals)
            })
        };
        let max0 = |key: &str| -> f64 {
            dw_keys
                .iter()
                .map(|&dw| v0d(dw, key))
                .fold(f64::NEG_INFINITY, f64::max)
        };

        // Build min_period table with SS cross-corner correction and monotone upper hull.
        // SS corner Liberate MX fails to converge at dw=1024/1536/2048; those entries
        // are replaced with TT * avg_ratio where ratio ≈ 1.828 across all valid dws.
        let min_period_snaps: Vec<(u32, [f64; 7])> = {
            let mut raw: Vec<(u32, [f64; 7])> = dw_keys
                .iter()
                .map(|&dw| (dw, std::array::from_fn(|j| v1d(dw, "min_period", j))))
                .collect();

            if corner == "ss" {
                if let Some(tt_map) = root["tt"].as_object() {
                    let v1d_tt = |dw: u32, j: usize| -> f64 {
                        tt_map[&dw.to_string()]["min_period"][j]
                            .as_f64()
                            .unwrap_or(0.0)
                    };
                    let mut ratios = Vec::new();
                    for &(dw, ref arr) in &raw {
                        if dw <= 768 {
                            let tt_val = v1d_tt(dw, 0);
                            if tt_val > 0.0 {
                                ratios.push(arr[0] / tt_val);
                            }
                        }
                    }
                    if !ratios.is_empty() {
                        let avg_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64;
                        for (dw, arr) in raw.iter_mut() {
                            if *dw > 768 {
                                for j in 0..7 {
                                    arr[j] = v1d_tt(*dw, j) * avg_ratio;
                                }
                            }
                        }
                    }
                }
            }

            // Monotone upper hull (running max left-to-right) applied to every corner.
            let mut running_max = [0.0f64; 7];
            for (_, arr) in raw.iter_mut() {
                for j in 0..7 {
                    running_max[j] = f64::max(running_max[j], arr[j]);
                    arr[j] = running_max[j];
                }
            }
            raw
        };

        Ok(Self {
            cell_rise: fit7("cell_rise", basis_lin4_sqrt_log),
            cell_fall: fit7("cell_fall", basis_lin4_sqrt_log),
            rise_transition: fit7("rise_transition", basis_lin3_cbrt),
            fall_transition: fit7("fall_transition", basis_lin3_sqrt),
            addr_hold_rise: fit7("addr_hold_rise", basis_lin3_sqrt),
            addr_hold_fall: fit7("addr_hold_fall", basis_log2),
            addr_setup_rise: fit7("addr_setup_rise", basis_lin3_sqrt),
            addr_setup_fall: fit7("addr_setup_fall", basis_lin3_sqrt),
            din_hold_rise: fit7("din_hold_rise", basis_lin2),
            din_hold_fall: max7("din_hold_fall"),
            din_setup_rise: fit7("din_setup_rise", basis_lin2),
            din_setup_fall: fit7("din_setup_fall", basis_lin2),
            ce_hold_rise: fit7("ce_hold_rise", basis_log3),
            ce_hold_fall: fit7("ce_hold_fall", basis_lin3_sqrt),
            ce_setup_rise: fit7("ce_setup_rise", basis_log3),
            ce_setup_fall: fit7("ce_setup_fall", basis_lin3_sqrt),
            we_hold_rise: fit7("we_hold_rise", basis_log3),
            we_hold_fall: fit7("we_hold_fall", basis_lin3_sqrt),
            we_setup_rise: fit7("we_setup_rise", basis_log3),
            we_setup_fall: fit7("we_setup_fall", basis_lin3_sqrt),
            rstb_hold_rise: fit7("rstb_hold_rise", basis_log3),
            rstb_hold_fall: fit7("rstb_hold_fall", basis_lin4_log_log2),
            rstb_setup_rise: fit7("rstb_setup_rise", basis_log3),
            rstb_setup_fall: fit7("rstb_setup_fall", basis_lin4_log_log2),
            mpw_rise: fit1("mpw_rise", basis_lin3_cbrt),
            mpw_fall: fit1("mpw_fall", basis_log3),
            min_period: min_period_snaps,
            clk_cap: max0("clk_cap"),
            addr_cap: max0("addr_cap"),
            din_cap: max0("din_cap"),
            ce_cap: max0("ce_cap"),
            we_cap: max0("we_cap"),
            rstb_cap: max0("rstb_cap"),
            wmask_cap: max0("wmask_cap"),
        })
    }
}

impl TimingModel for LookupModel {
    fn area(&self, _: &SramParams) -> f64 {
        0.0
    }

    fn hold_rise(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7] {
        let dw = p.data_width() as f64;
        let tbl = match pin {
            PinRole::Addr | PinRole::Clk => &self.addr_hold_rise,
            PinRole::Din | PinRole::Wmask => &self.din_hold_rise,
            PinRole::Ce => &self.ce_hold_rise,
            PinRole::We => &self.we_hold_rise,
            PinRole::Rstb => &self.rstb_hold_rise,
        };
        eval_7x7(tbl, dw)
    }

    fn hold_fall(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7] {
        let dw = p.data_width() as f64;
        match pin {
            PinRole::Din | PinRole::Wmask => self.din_hold_fall,
            PinRole::Addr | PinRole::Clk => eval_7x7(&self.addr_hold_fall, dw),
            PinRole::Ce => eval_7x7(&self.ce_hold_fall, dw),
            PinRole::We => eval_7x7(&self.we_hold_fall, dw),
            PinRole::Rstb => eval_7x7(&self.rstb_hold_fall, dw),
        }
    }

    fn setup_rise(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7] {
        let dw = p.data_width() as f64;
        match pin {
            PinRole::Addr | PinRole::Clk => eval_7x7(&self.addr_setup_rise, dw),
            PinRole::Din | PinRole::Wmask => eval_7x7(&self.din_setup_rise, dw),
            PinRole::Ce => eval_7x7(&self.ce_setup_rise, dw),
            PinRole::We => eval_7x7(&self.we_setup_rise, dw),
            PinRole::Rstb => eval_7x7(&self.rstb_setup_rise, dw),
        }
    }
    fn setup_fall(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7] {
        let dw = p.data_width() as f64;
        match pin {
            PinRole::Addr | PinRole::Clk => eval_7x7(&self.addr_setup_fall, dw),
            PinRole::Din | PinRole::Wmask => eval_7x7(&self.din_setup_fall, dw),
            PinRole::Ce => eval_7x7(&self.ce_setup_fall, dw),
            PinRole::We => eval_7x7(&self.we_setup_fall, dw),
            PinRole::Rstb => eval_7x7(&self.rstb_setup_fall, dw),
        }
    }

    fn cell_rise(&self, p: &SramParams) -> [[f64; 7]; 7] {
        eval_7x7(&self.cell_rise, p.data_width() as f64)
    }
    fn cell_fall(&self, p: &SramParams) -> [[f64; 7]; 7] {
        eval_7x7(&self.cell_fall, p.data_width() as f64)
    }
    fn rise_transition(&self, p: &SramParams) -> [[f64; 7]; 7] {
        eval_7x7(&self.rise_transition, p.data_width() as f64)
    }
    fn fall_transition(&self, p: &SramParams) -> [[f64; 7]; 7] {
        eval_7x7(&self.fall_transition, p.data_width() as f64)
    }

    fn minimum_period(&self, p: &SramParams) -> [f64; 7] {
        let dw = p.data_width() as f64;
        let snaps = &self.min_period;
        let n = snaps.len();
        if n == 0 {
            return [0.0; 7];
        }
        let i = snaps.partition_point(|(d, _)| (*d as f64) <= dw);
        if i == 0 {
            return snaps[0].1;
        }
        if i >= n {
            return snaps[n - 1].1;
        }
        let (x0, y0) = snaps[i - 1];
        let (x1, y1) = snaps[i];
        let t = (dw.ln() - (x0 as f64).ln()) / ((x1 as f64).ln() - (x0 as f64).ln());
        std::array::from_fn(|j| y0[j] + t * (y1[j] - y0[j]))
    }

    fn min_pulse_width(&self, p: &SramParams) -> ([f64; 7], [f64; 7]) {
        let dw = p.data_width() as f64;
        let rise = std::array::from_fn(|j| self.mpw_rise[j].eval(dw));
        let fall = std::array::from_fn(|j| self.mpw_fall[j].eval(dw));
        (rise, fall)
    }

    fn input_cap(&self, _: &SramParams, pin: PinRole) -> CapValues {
        let cap = match pin {
            PinRole::Clk => self.clk_cap,
            PinRole::Addr => self.addr_cap,
            PinRole::Din => self.din_cap,
            PinRole::Ce => self.ce_cap,
            PinRole::We => self.we_cap,
            PinRole::Rstb => self.rstb_cap,
            PinRole::Wmask => self.wmask_cap,
        };
        CapValues {
            cap,
            rise: cap,
            fall: cap,
            rise_lo: 0.0,
            fall_lo: 0.0,
        }
    }

    fn output_max_cap(&self, _: &SramParams) -> f64 {
        0.52
    }
    fn receiver_cap_1_rise(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }
    fn receiver_cap_2_rise(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }
    fn receiver_cap_1_fall(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }
    fn receiver_cap_2_fall(&self, _: &SramParams, _: PinRole) -> [f64; 7] {
        [0.0; 7]
    }

    fn ccs_rise(&self, _: &SramParams) -> Vec<CcsVector> {
        vec![]
    }
    fn ccs_fall(&self, _: &SramParams) -> Vec<CcsVector> {
        vec![]
    }
    fn clk_power_rise(&self, _: &SramParams, _: &str) -> [f64; 7] {
        [0.0; 7]
    }
    fn clk_power_fall(&self, _: &SramParams, _: &str) -> [f64; 7] {
        [0.0; 7]
    }
}

// ── PUBLIC ENTRY POINT ────────────────────────────────────────────────────────

/// Mirrors `liberate_mx::LibParams` as a single parameter bundle.
pub struct LibGenParams<'a> {
    pub sram: &'a SramParams,
    pub pvt: PvtCorner,
    pub model: &'a dyn TimingModel,
    pub output: PathBuf,
}

/// Mirrors `liberate_mx::generate_sram_lib` — writes a `.lib` file and returns its path.
pub fn generate_sram_lib(params: &LibGenParams) -> Result<PathBuf> {
    let content = write_liberty(params.sram, &params.pvt, params.model);
    if let Some(parent) = params.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&params.output, content)?;
    Ok(params.output.clone())
}

// ── WRITER ────────────────────────────────────────────────────────────────────

struct W {
    buf: String,
    indent: usize,
}

impl W {
    fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
        }
    }

    fn ln(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    fn block(&mut self, header: &str, f: impl FnOnce(&mut Self)) {
        self.ln(&format!("{} {{", header));
        self.indent += 1;
        f(self);
        self.indent -= 1;
        self.ln("}");
    }

    fn attr(&mut self, k: &str, v: &str) {
        self.ln(&format!("{} : {};", k, v));
    }

    fn attr_q(&mut self, k: &str, v: &str) {
        self.ln(&format!("{} : \"{}\";", k, v));
    }

    fn attr_f(&mut self, k: &str, v: f64) {
        self.ln(&format!("{} : {};", k, fmtf(v)));
    }

    fn lut_2d(
        &mut self,
        name: &str,
        tmpl: &str,
        idx1: &[f64; 7],
        idx2: &[f64; 7],
        rows: &[[f64; 7]; 7],
    ) {
        self.block(&format!("{} ({})", name, tmpl), |w| {
            w.ln(&format!("index_1 (\"{}\");", fmt_idx(idx1)));
            w.ln(&format!("index_2 (\"{}\");", fmt_idx(idx2)));
            w.ln("values ( \\");
            w.indent += 1;
            for (i, row) in rows.iter().enumerate() {
                let vals: Vec<String> = row.iter().map(|v| fmtf(*v)).collect();
                let trail = if i < 6 { " \\" } else { "" };
                w.ln(&format!("\"{}\"{}", vals.join(", "), trail));
            }
            w.indent -= 1;
            w.ln(");");
        });
    }

    fn lut_1d(&mut self, name: &str, tmpl: &str, idx1: &[f64; 7], values: &[f64; 7]) {
        self.block(&format!("{} ({})", name, tmpl), |w| {
            w.ln(&format!("index_1 (\"{}\");", fmt_idx(idx1)));
            w.ln("values ( \\");
            w.indent += 1;
            let vals: Vec<String> = values.iter().map(|v| fmtf(*v)).collect();
            w.ln(&format!("\"{}\" \\", vals.join(", ")));
            w.indent -= 1;
            w.ln(");");
        });
    }

    fn receiver_cap(&mut self, p: &SramParams, m: &dyn TimingModel, pin: PinRole) {
        let r1r = m.receiver_cap_1_rise(p, pin);
        let r2r = m.receiver_cap_2_rise(p, pin);
        let r1f = m.receiver_cap_1_fall(p, pin);
        let r2f = m.receiver_cap_2_fall(p, pin);
        self.block("receiver_capacitance ()", |w| {
            w.lut_1d(
                "receiver_capacitance1_rise",
                "receiver_cap_power_template_7x7",
                &SLEW_IDX,
                &r1r,
            );
            w.lut_1d(
                "receiver_capacitance2_rise",
                "receiver_cap_power_template_7x7",
                &SLEW_IDX,
                &r2r,
            );
            w.lut_1d(
                "receiver_capacitance1_fall",
                "receiver_cap_power_template_7x7",
                &SLEW_IDX,
                &r1f,
            );
            w.lut_1d(
                "receiver_capacitance2_fall",
                "receiver_cap_power_template_7x7",
                &SLEW_IDX,
                &r2f,
            );
        });
    }

    fn constraint_timing(
        &mut self,
        p: &SramParams,
        m: &dyn TimingModel,
        pin: PinRole,
        ttype: &str,
    ) {
        let (rise, fall) = if ttype == "hold_rising" {
            (m.hold_rise(p, pin), m.hold_fall(p, pin))
        } else {
            (m.setup_rise(p, pin), m.setup_fall(p, pin))
        };
        self.block("timing ()", |w| {
            w.attr_q("related_pin", "clk");
            w.attr("sdf_edges", "both_edges");
            w.attr("timing_type", ttype);
            w.lut_2d(
                "rise_constraint",
                "constraint_template_7x7",
                &SLEW_IDX,
                &SLEW_IDX,
                &rise,
            );
            w.lut_2d(
                "fall_constraint",
                "constraint_template_7x7",
                &SLEW_IDX,
                &SLEW_IDX,
                &fall,
            );
        });
    }

    fn input_pin_attrs(&mut self, cap: &CapValues) {
        self.attr("related_ground_pin", "vss");
        self.attr("related_power_pin", "vdd");
        self.attr_f("max_transition", 0.351);
        self.attr_f("capacitance", cap.cap);
        self.attr_f("rise_capacitance", cap.rise);
        self.ln(&format!(
            "rise_capacitance_range ({}, {});",
            fmtf(cap.rise_lo),
            fmtf(cap.rise)
        ));
        self.attr_f("fall_capacitance", cap.fall);
        self.ln(&format!(
            "fall_capacitance_range ({}, {});",
            fmtf(cap.fall_lo),
            fmtf(cap.fall)
        ));
    }

    fn ccs_arcs(&mut self, rise_or_fall: &str, vecs: &[CcsVector]) {
        self.block(&format!("output_current_{} ()", rise_or_fall), |w| {
            for vec in vecs {
                w.block("vector (ccs_template)", |w| {
                    w.attr_f("reference_time", vec.reference_time);
                    w.ln(&format!("index_1 (\"{}\");", fmtf(vec.slew)));
                    w.ln(&format!("index_2 (\"{}\");", fmtf(vec.load)));
                    let tpts: Vec<String> = vec.time_pts.iter().map(|v| fmtf(*v)).collect();
                    w.ln(&format!("index_3 (\"{}\");", tpts.join(", ")));
                    let curr: Vec<String> = vec.currents.iter().map(|v| fmtf(*v)).collect();
                    w.ln(&format!("values ( \\"));
                    w.indent += 1;
                    w.ln(&format!("\"{}\" \\", curr.join(", ")));
                    w.indent -= 1;
                    w.ln(");");
                });
            }
        });
    }
}

// ── HELPERS ───────────────────────────────────────────────────────────────────

fn fmtf(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if v.fract() == 0.0 && v.abs() < 1e10 {
        return format!("{}", v as i64);
    }
    let s = format!("{:.7}", v);
    let s = s.trim_end_matches('0');
    s.trim_end_matches('.').to_string()
}

fn fmt_idx(idx: &[f64; 7]) -> String {
    idx.iter().map(|v| fmtf(*v)).collect::<Vec<_>>().join(", ")
}

/// 1.95 → "1P95", 1.8 → "1P8", 1.6 → "1P6"
fn fmt_voltage_oc(v: f64) -> String {
    let s = format!("{:.2}", v);
    let (int, dec) = s.split_once('.').unwrap();
    let dec = dec.trim_end_matches('0');
    if dec.is_empty() {
        format!("{}P", int)
    } else {
        format!("{}P{}", int, dec)
    }
}

fn write_bus_type(w: &mut W, cell: &str, signal: &str, hi: usize, lo: usize) {
    let name = format!("bus_{}_{}_{}_{}", cell, signal, hi, lo);
    w.block(&format!("type ({})", name), |w| {
        w.attr("base_type", "array");
        w.attr("data_type", "bit");
        w.attr("bit_width", &(hi - lo + 1).to_string());
        w.attr("bit_from", &hi.to_string());
        w.attr("bit_to", &lo.to_string());
        w.attr("downto", "true");
    });
}

fn write_lu_template(
    w: &mut W,
    name: &str,
    var1: &str,
    var2: Option<&str>,
    idx1: &[f64; 7],
    idx2: Option<&[f64; 7]>,
) {
    w.block(&format!("lu_table_template ({})", name), |w| {
        w.attr("variable_1", var1);
        if let Some(v2) = var2 {
            w.attr("variable_2", v2);
        }
        w.ln(&format!("index_1 (\"{}\");", fmt_idx(idx1)));
        if let Some(i2) = idx2 {
            w.ln(&format!("index_2 (\"{}\");", fmt_idx(i2)));
        }
    });
}

fn write_power_template(
    w: &mut W,
    name: &str,
    var1: &str,
    var2: Option<&str>,
    idx1: &[f64; 7],
    idx2: Option<&[f64; 7]>,
) {
    w.block(&format!("power_lut_template ({})", name), |w| {
        w.attr("variable_1", var1);
        if let Some(v2) = var2 {
            w.attr("variable_2", v2);
        }
        w.ln(&format!("index_1 (\"{}\");", fmt_idx(idx1)));
        if let Some(i2) = idx2 {
            w.ln(&format!("index_2 (\"{}\");", fmt_idx(i2)));
        }
    });
}

fn write_control_pin(w: &mut W, p: &SramParams, m: &dyn TimingModel, name: &str, role: PinRole) {
    let cap = m.input_cap(p, role);
    w.block(&format!("pin ({})", name), |w| {
        w.attr("direction", "input");
        w.input_pin_attrs(&cap);
        w.constraint_timing(p, m, role, "hold_rising");
        w.constraint_timing(p, m, role, "setup_rising");
        w.receiver_cap(p, m, role);
    });
}

// ── TOP-LEVEL LIBERTY SERIALIZER ──────────────────────────────────────────────

fn write_liberty(p: &SramParams, pvt: &PvtCorner, m: &dyn TimingModel) -> String {
    let mut w = W::new();
    let cell_name = p.name();
    let cell = cell_name.as_str();
    let oc = pvt.oc_name();
    let addr_hi = p.addr_width() - 1;
    let data_hi = p.data_width() - 1;
    let wmask_width = p.wmask_width();

    w.block(&format!("library ({})", cell), |w| {
        // Library-level scalar attributes (order matches Liberate MX output)
        w.ln("/* Generated by sram22 open-source lib generator */");
        w.attr("delay_model", "table_lookup");
        w.attr_q("comment", "");
        w.attr_q("revision", "1.0");
        w.ln("capacitive_load_unit (1,pf);");
        w.attr_q("current_unit", "1mA");
        w.attr_q("leakage_power_unit", "1nW");
        w.attr_q("pulling_resistance_unit", "1kohm");
        w.attr_q("time_unit", "1ns");
        w.attr_q("voltage_unit", "1V");
        w.attr("default_cell_leakage_power", "0");
        w.attr("default_fanout_load", "1");
        w.attr("default_inout_pin_cap", "0.005");
        w.attr("default_input_pin_cap", "0.005");
        w.attr("default_leakage_power_density", "0");
        w.attr("default_max_transition", "0.04");
        w.attr("default_output_pin_cap", "0");
        w.attr("in_place_swap_mode", "match_footprint");
        w.attr("input_threshold_pct_fall", "50");
        w.attr("input_threshold_pct_rise", "50");
        w.attr_f("nom_process", pvt.process);
        w.attr_f("nom_temperature", pvt.temperature);
        w.attr_f("nom_voltage", pvt.voltage);
        w.attr("output_threshold_pct_fall", "50");
        w.attr("output_threshold_pct_rise", "50");
        w.attr("slew_derate_from_library", "1");
        w.attr("slew_lower_threshold_pct_fall", "10");
        w.attr("slew_lower_threshold_pct_rise", "10");
        w.attr("slew_upper_threshold_pct_fall", "90");
        w.attr("slew_upper_threshold_pct_rise", "90");
        w.ln(&format!("voltage_map (vdd, {});", fmtf(pvt.voltage)));
        w.ln("voltage_map (vss, 0);");
        w.ln("voltage_map (GND, 0);");

        // Operating conditions
        w.block(&format!("operating_conditions ({})", oc), |w| {
            w.attr_f("process", pvt.process);
            w.attr_f("temperature", pvt.temperature);
            w.attr_f("voltage", pvt.voltage);
        });
        w.attr_q("default_operating_conditions", &oc);
        w.attr_q("bus_naming_style", "%s[%d]");

        // Bus type definitions
        if wmask_width > 1 {
            write_bus_type(w, cell, "wmask", wmask_width - 1, 0);
        }
        write_bus_type(w, cell, "addr", addr_hi, 0);
        write_bus_type(w, cell, "din", data_hi, 0);
        write_bus_type(w, cell, "dout", data_hi, 0);

        // Lookup table templates
        w.block("output_current_template (ccs_template)", |w| {
            w.attr("variable_1", "input_net_transition");
            w.attr("variable_2", "total_output_net_capacitance");
            w.attr("variable_3", "time");
        });
        write_lu_template(
            w,
            "constraint_template_7x7",
            "constrained_pin_transition",
            Some("related_pin_transition"),
            &SLEW_IDX,
            Some(&SLEW_IDX),
        );
        write_lu_template(
            w,
            "delay_template_7x7",
            "input_net_transition",
            Some("total_output_net_capacitance"),
            &SLEW_IDX,
            Some(&LOAD_IDX),
        );
        write_lu_template(
            w,
            "mpw_constraint_template_7x7",
            "constrained_pin_transition",
            None,
            &SLEW_IDX,
            None,
        );
        write_power_template(
            w,
            "passive_output_power_template_7x1",
            "total_output_net_capacitance",
            None,
            &PWR_IDX,
            None,
        );
        write_power_template(
            w,
            "passive_power_template_7x1",
            "input_transition_time",
            None,
            &SLEW_IDX,
            None,
        );
        write_power_template(
            w,
            "power_template_7x7",
            "input_transition_time",
            Some("total_output_net_capacitance"),
            &SLEW_IDX,
            Some(&PWR_IDX),
        );
        write_lu_template(
            w,
            "receiver_cap_power_template_7x7",
            "input_net_transition",
            None,
            &SLEW_IDX,
            None,
        );

        w.ln("define (char_when, receiver_capacitance, string);");
        w.ln("define (is_propagating, receiver_capacitance, string);");

        // ── Cell ─────────────────────────────────────────────────────────────
        w.block(&format!("cell ({})", cell), |w| {
            w.attr_f("area", m.area(p));
            w.attr("cell_leakage_power", "0");
            w.attr("dont_touch", "true");
            w.attr("dont_use", "true");
            w.attr("interface_timing", "true");
            w.attr("is_macro_cell", "true");

            w.block("pg_pin (vdd)", |w| {
                w.attr("direction", "inout");
                w.attr("pg_type", "primary_power");
                w.attr_q("voltage_name", "vdd");
            });
            w.block("pg_pin (vss)", |w| {
                w.attr("direction", "inout");
                w.attr("pg_type", "primary_ground");
                w.attr_q("voltage_name", "vss");
            });
            w.block("memory ()", |w| {
                w.attr("address_width", &p.addr_width().to_string());
                w.attr("type", "ram");
                w.attr("word_width", &p.data_width().to_string());
            });

            // bus(addr)
            let addr_bus_type = format!("bus_{}_addr_{}_{}", cell, addr_hi, 0);
            let small_cap = m.input_cap(p, PinRole::Addr);
            w.block("bus (addr)", |w| {
                w.attr_q("bus_type", &addr_bus_type);
                w.attr("direction", "input");
                for bit in (0..=addr_hi).rev() {
                    w.block(&format!("pin (addr[{}])", bit), |w| {
                        w.input_pin_attrs(&small_cap);
                    });
                }
                w.constraint_timing(p, m, PinRole::Addr, "hold_rising");
                w.constraint_timing(p, m, PinRole::Addr, "setup_rising");
                w.receiver_cap(p, m, PinRole::Addr);
            });

            // pin(ce)
            write_control_pin(w, p, m, "ce", PinRole::Ce);

            // pin(clk)  — clock : true, larger capacitance
            let clk_cap = m.input_cap(p, PinRole::Clk);
            let (mpw_rise, mpw_fall) = m.min_pulse_width(p);
            let min_period = m.minimum_period(p);
            let when_conds: &[(&str, &str)] = &[
                ("we&ce", "vdd"),
                ("we&ce", "vss"),
                ("we&!ce", "vdd"),
                ("we&!ce", "vss"),
                ("!we&ce", "vdd"),
                ("!we&ce", "vss"),
                ("!we&!ce", "vdd"),
                ("!we&!ce", "vss"),
            ];
            w.block("pin (clk)", |w| {
                w.attr("clock", "true");
                w.attr("direction", "input");
                w.attr("related_ground_pin", "vss");
                w.attr("related_power_pin", "vdd");
                w.attr_f("max_transition", 0.351);
                w.attr_f("capacitance", clk_cap.cap);
                w.attr_f("rise_capacitance", clk_cap.rise);
                w.ln(&format!(
                    "rise_capacitance_range ({}, {});",
                    fmtf(clk_cap.rise_lo),
                    fmtf(clk_cap.rise)
                ));
                w.attr_f("fall_capacitance", clk_cap.fall);
                w.ln(&format!(
                    "fall_capacitance_range ({}, {});",
                    fmtf(clk_cap.fall_lo),
                    fmtf(clk_cap.fall)
                ));
                w.block("timing ()", |w| {
                    w.attr_q("related_pin", "clk");
                    w.attr("timing_type", "min_pulse_width");
                    w.lut_1d(
                        "rise_constraint",
                        "mpw_constraint_template_7x7",
                        &SLEW_IDX,
                        &mpw_rise,
                    );
                    w.lut_1d(
                        "fall_constraint",
                        "mpw_constraint_template_7x7",
                        &SLEW_IDX,
                        &mpw_fall,
                    );
                });
                w.block("timing ()", |w| {
                    w.attr_q("related_pin", "clk");
                    w.attr("timing_type", "minimum_period");
                    w.lut_1d(
                        "rise_constraint",
                        "mpw_constraint_template_7x7",
                        &SLEW_IDX,
                        &min_period,
                    );
                    w.lut_1d(
                        "fall_constraint",
                        "mpw_constraint_template_7x7",
                        &SLEW_IDX,
                        &min_period,
                    );
                });
                for &(when, pg) in when_conds {
                    let rise_pwr = m.clk_power_rise(p, when);
                    let fall_pwr = m.clk_power_fall(p, when);
                    w.block("internal_power ()", |w| {
                        w.attr_q("when", when);
                        w.attr_q("related_pg_pin", pg);
                        w.lut_1d(
                            "rise_power",
                            "passive_power_template_7x1",
                            &SLEW_IDX,
                            &rise_pwr,
                        );
                        w.lut_1d(
                            "fall_power",
                            "passive_power_template_7x1",
                            &SLEW_IDX,
                            &fall_pwr,
                        );
                    });
                }
            });

            // bus(din)
            let din_bus_type = format!("bus_{}_din_{}_{}", cell, data_hi, 0);
            let din_cap = m.input_cap(p, PinRole::Din);
            w.block("bus (din)", |w| {
                w.attr_q("bus_type", &din_bus_type);
                w.attr("direction", "input");
                for bit in (0..=data_hi).rev() {
                    w.block(&format!("pin (din[{}])", bit), |w| {
                        w.input_pin_attrs(&din_cap);
                    });
                }
                w.constraint_timing(p, m, PinRole::Din, "hold_rising");
                w.constraint_timing(p, m, PinRole::Din, "setup_rising");
                w.receiver_cap(p, m, PinRole::Din);
            });

            // bus(dout) — timing at bus level, not per-pin
            let dout_bus_type = format!("bus_{}_dout_{}_{}", cell, data_hi, 0);
            let cell_r = m.cell_rise(p);
            let cell_f = m.cell_fall(p);
            let rise_tr = m.rise_transition(p);
            let fall_tr = m.fall_transition(p);
            let ccs_r = m.ccs_rise(p);
            let ccs_f = m.ccs_fall(p);
            let out_max_cap = m.output_max_cap(p);
            w.block("bus (dout)", |w| {
                w.attr_q("bus_type", &dout_bus_type);
                w.attr("direction", "output");
                for bit in (0..=data_hi).rev() {
                    w.block(&format!("pin (dout[{}])", bit), |w| {
                        w.attr_q("power_down_function", "(!vdd) + (vss)");
                        w.attr("related_ground_pin", "vss");
                        w.attr("related_power_pin", "vdd");
                        w.attr_f("max_capacitance", out_max_cap);
                    });
                }
                w.block("timing ()", |w| {
                    w.attr_q("related_pin", "clk");
                    w.attr_q("sdf_cond", "ce == 1'b1");
                    w.attr("timing_sense", "non_unate");
                    w.attr("timing_type", "rising_edge");
                    w.attr_q("when", "ce");
                    w.lut_2d(
                        "cell_rise",
                        "delay_template_7x7",
                        &SLEW_IDX,
                        &LOAD_IDX,
                        &cell_r,
                    );
                    w.lut_2d(
                        "rise_transition",
                        "delay_template_7x7",
                        &SLEW_IDX,
                        &LOAD_IDX,
                        &rise_tr,
                    );
                    w.lut_2d(
                        "cell_fall",
                        "delay_template_7x7",
                        &SLEW_IDX,
                        &LOAD_IDX,
                        &cell_f,
                    );
                    w.lut_2d(
                        "fall_transition",
                        "delay_template_7x7",
                        &SLEW_IDX,
                        &LOAD_IDX,
                        &fall_tr,
                    );
                    if !ccs_r.is_empty() {
                        w.ccs_arcs("rise", &ccs_r);
                    }
                    if !ccs_f.is_empty() {
                        w.ccs_arcs("fall", &ccs_f);
                    }
                });
            });

            // pin(rstb)
            write_control_pin(w, p, m, "rstb", PinRole::Rstb);

            // pin(we)
            write_control_pin(w, p, m, "we", PinRole::We);

            // bus(wmask) — 2-bit bus or scalar pin depending on wmask_width
            if wmask_width > 1 {
                let wmask_bus_type = format!("bus_{}_wmask_{}_{}", cell, wmask_width - 1, 0);
                let wmask_cap = m.input_cap(p, PinRole::Wmask);
                w.block("bus (wmask)", |w| {
                    w.attr_q("bus_type", &wmask_bus_type);
                    w.attr("direction", "input");
                    for bit in (0..wmask_width).rev() {
                        w.block(&format!("pin (wmask[{}])", bit), |w| {
                            w.input_pin_attrs(&wmask_cap);
                        });
                    }
                    w.constraint_timing(p, m, PinRole::Wmask, "hold_rising");
                    w.constraint_timing(p, m, PinRole::Wmask, "setup_rising");
                    w.receiver_cap(p, m, PinRole::Wmask);
                });
            } else {
                write_control_pin(w, p, m, "wmask", PinRole::Wmask);
            }
        });
    });

    w.buf
}
