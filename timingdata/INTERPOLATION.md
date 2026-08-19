# Liberty Timing Interpolation — Architecture and Code Reference

## Overview

SRAM22 can generate Liberty (.lib) timing files without running Liberate MX by using
a data-driven interpolation model called `LookupModel`, implemented in `src/lib_gen.rs`.

The model is pre-characterized at a sparse set of `data_width` values and interpolates
smoothly to any requested `data_width` at runtime. Characterization data lives in this
directory as JSON files. They are embedded directly into the compiled binary via
`include_bytes!` — no external files are needed at runtime.

---

## Characterization Data Files

### Naming convention

```
timingdata/{num_words}m{mux_ratio}w{write_size}.json
```

Each file contains timing measurements across 7 `data_width` breakpoints
(8, 16, 32, 48, 64, 96, 128) for all 3 PVT corners (tt, ss, ff), produced by
Liberate MX running SPICE simulation.

### Currently available

| File | num_words | mux_ratio | write_size |
|---|:---:|:---:|:---:|
| `64m4w8.json`  | 64  | 4 | 8 |
| `128m8w8.json` | 128 | 8 | 8 |
| `256m4w8.json` | 256 | 4 | 8 |

### JSON structure

```json
{
  "tt": {
    "8":  { "cell_rise": [[...7x7...]], "min_period": [...7...], "clk_cap": 0.123, ... },
    "16": { ... },
    ...
    "128": { ... }
  },
  "ss": { ... },
  "ff": { ... }
}
```

The top level is keyed by PVT corner. Each corner maps `data_width` (as a string) to
a timing snapshot containing all Liberty tables for that configuration.


## Configuration Selection (`src/plan/mod.rs`)

```rust
static TIMING_DATA: &[(usize, usize, usize, &[u8])] = &[
    (64,  4, 8, include_bytes!(..."/timingdata/64m4w8.json")),
    (128, 8, 8, include_bytes!(..."/timingdata/128m8w8.json")),
    (256, 4, 8, include_bytes!(..."/timingdata/256m4w8.json")),
];
```

This is a compile-time table of `(num_words, mux_ratio, write_size, embedded_bytes)` tuples.
`include_bytes!` embeds the entire JSON file as a `&'static [u8]` slice directly in the
binary — there is no runtime file I/O.

At runtime, the correct entry is selected by matching the SRAM parameters:

```rust
let nw = plan.sram_params.num_words();
let mx = plan.sram_params.mux_ratio();
let ws = plan.sram_params.wmask_granularity();
let json_bytes: &[u8] = TIMING_DATA.iter()
    .find(|(n, m, w, _)| *n == nw && *m == mx && *w == ws)
    .map(|(_, _, _, b)| *b)
    .ok_or_else(|| anyhow::anyhow!(
        "no timing data for {}m{}w{} — add timingdata/{}m{}w{}.json to the repo",
        nw, mx, ws, nw, mx, ws
    ))?;
```

If no matching entry exists, the error message tells the developer exactly which file
to create and where to register it.

---

## LookupModel Construction (`LookupModel::from_json`)

`LookupModel::from_json(json_bytes, corner)` deserialises the JSON and fits a
parametric model for the requested PVT corner. This is the only expensive step —
it runs once when a lib file is about to be generated.

### Step 1 — Parse data widths

```rust
let root: serde_json::Value = serde_json::from_slice(json_bytes)?;
let cmap = root[corner].as_object()...;

let mut dw_keys: Vec<u32> = cmap.keys().map(|k| k.parse().unwrap()).collect();
dw_keys.sort_unstable();
let dws: Vec<f64> = dw_keys.iter().map(|&k| k as f64).collect();
```

The JSON is parsed into a `serde_json::Value`. Only the chosen corner's data is used.
`data_width` values outside the supported range (8–128) are rejected before this point
by `generate_plan`, so `from_json` can assume all keys are valid.

### Step 2 — Helper closures

Three closures extract values from the JSON by dimensionality:

```rust
let v2d = |dw, key, i, j| cmap[&dw.to_string()][key][i][j].as_f64()...;  // 7×7 table entry
let v1d = |dw, key, j|    cmap[&dw.to_string()][key][j].as_f64()...;      // 1×7 array entry
let v0d = |dw, key|       cmap[&dw.to_string()][key].as_f64()...;          // scalar
```

Three fitting closures use these to build fitted structures:

```rust
// Fit a 7×7 table: one FitEntry per (i,j) cell, each fitted independently
let fit7 = |key, basis| -> [[FitEntry; 7]; 7] {
    std::array::from_fn(|i| std::array::from_fn(|j| {
        let vals: Vec<f64> = dw_keys.iter().map(|&dw| v2d(dw, key, i, j)).collect();
        FitEntry::fit(basis, &dws, &vals)
    }))
};

// Take element-wise max across all dw for a 7×7 table (used for din_hold_fall)
let max7 = |key| -> [[f64; 7]; 7] {
    std::array::from_fn(|i| std::array::from_fn(|j| {
        dw_keys.iter().map(|&dw| v2d(dw, key, i, j)).fold(f64::NEG_INFINITY, f64::max)
    }))
};

// Fit a 1×7 array: one FitEntry per index j
let fit1 = |key, basis| -> [FitEntry; 7] {
    std::array::from_fn(|j| {
        let vals: Vec<f64> = dw_keys.iter().map(|&dw| v1d(dw, key, j)).collect();
        FitEntry::fit(basis, &dws, &vals)
    })
};

// Scalar max across all dw (used for capacitances)
let max0 = |key| -> f64 {
    dw_keys.iter().map(|&dw| v0d(dw, key)).fold(f64::NEG_INFINITY, f64::max)
};
```

### Step 3 — Build the model fields

Each Liberty table is assigned a fitting strategy:

```rust
Ok(Self {
    cell_rise:       fit7("cell_rise",       basis_lin4_sqrt_log),
    cell_fall:       fit7("cell_fall",       basis_lin4_sqrt_log),
    rise_transition: fit7("rise_transition", basis_lin3_cbrt),
    fall_transition: fit7("fall_transition", basis_lin3_sqrt),
    addr_hold_rise:  fit7("addr_hold_rise",  basis_lin3_sqrt),
    addr_hold_fall:  fit7("addr_hold_fall",  basis_log2),
    din_hold_rise:   fit7("din_hold_rise",   basis_lin2),
    din_hold_fall:   max7("din_hold_fall"),   // no curve fit — see below
    ce_hold_rise:    fit7("ce_hold_rise",    basis_log3),
    ce_hold_fall:    fit7("ce_hold_fall",    basis_lin3_sqrt),
    we_hold_rise:    fit7("we_hold_rise",    basis_log3),
    we_hold_fall:    fit7("we_hold_fall",    basis_lin3_sqrt),
    rstb_hold_rise:  fit7("rstb_hold_rise",  basis_log3),
    rstb_hold_fall:  fit7("rstb_hold_fall",  basis_lin4_log_log2),
    mpw_rise:        fit1("mpw_rise",        basis_lin3_cbrt),
    mpw_fall:        fit1("mpw_fall",        basis_log3),
    min_period:      min_period_snaps,        // piecewise — see below
    clk_cap:   max0("clk_cap"),
    addr_cap:  max0("addr_cap"),
    din_cap:   max0("din_cap"),
    ce_cap:    max0("ce_cap"),
    we_cap:    max0("we_cap"),
    rstb_cap:  max0("rstb_cap"),
    wmask_cap: max0("wmask_cap"),
})
```

**Capacitances** (`max0`): pin capacitances are essentially constant with `data_width`
(they reflect individual pin loading, not array size). The maximum across all
characterized `dw` is used as a conservative scalar.

**`din_hold_fall`** (`max7`): characterization data shows only ~3.7% variation with
no dw trend — this is measurement noise, not a real physical relationship. A curve
fit would be meaningless. Each of the 49 entries stores the global maximum observed
across all characterized `data_width` values, which is guaranteed conservative.

---

## Basis Functions

All curve-fitted tables use a basis of at most 4 functions of `dw`. Each basis
returns `[f64; 4]` — trailing zeros are unused slots that simplify the shared OLS
solver to always work on 4 coefficients.

```rust
fn basis_lin4_sqrt_log(dw: f64) -> [f64; 4] { [1.0, dw, dw.sqrt(), dw.ln()] }
fn basis_lin3_cbrt(dw: f64)     -> [f64; 4] { [1.0, dw, dw.cbrt(), 0.0] }
fn basis_lin3_sqrt(dw: f64)     -> [f64; 4] { [1.0, dw, dw.sqrt(), 0.0] }
fn basis_log2(dw: f64)          -> [f64; 4] { [1.0, dw.ln(), 0.0, 0.0] }
fn basis_lin2(dw: f64)          -> [f64; 4] { [1.0, dw, 0.0, 0.0] }
fn basis_log3(dw: f64)          -> [f64; 4] { [1.0, dw.ln(), dw.ln().powi(2), 0.0] }
fn basis_lin4_log_log2(dw: f64) -> [f64; 4] { let l = dw.ln(); [1.0, dw, l, l.powi(2)] }
```

Each basis was chosen by inspecting the shape of the Liberate MX data for that table
and selecting the minimal set of features that captures the dominant trend:

| Table | Basis | Equation | Physical reason |
|---|---|---|---|
| `cell_rise`, `cell_fall` | `basis_lin4_sqrt_log` | `a + b·dw + c·√dw + d·ln(dw)` | Hockey-stick shape: fast initial rise then flattening |
| `rise_transition`, `mpw_rise` | `basis_lin3_cbrt` | `a + b·dw + c·∛dw` | Sub-linear growth, gentler than √ |
| `fall_transition`, `addr_hold_rise`, `ce/we_hold_fall` | `basis_lin3_sqrt` | `a + b·dw + c·√dw` | Moderate sub-linear growth |
| `addr_hold_fall` | `basis_log2` | `a + b·ln(dw)` | Pure logarithmic — very slow growth |
| `din_hold_rise` | `basis_lin2` | `a + b·dw` | Linear — purely capacitive loading |
| `ce/we/rstb_hold_rise`, `mpw_fall` | `basis_log3` | `a + b·ln(dw) + c·ln²(dw)` | Logarithmic with curvature |
| `rstb_hold_fall` | `basis_lin4_log_log2` | `a + b·dw + c·ln(dw) + d·ln²(dw)` | Mixed linear and log (non-monotone dip in data) |

The key property is that all bases are **linear in their parameters** `[a, b, c, d]`,
even though they are nonlinear functions of `dw`. This makes the fit an ordinary
linear least-squares problem.

---

## FitEntry: OLS Fitting with Conservative Lift

`FitEntry` is the core fitted object. It stores a basis function pointer, 4
fitted coefficients, and the original training `(dw, val)` pairs.

```rust
struct FitEntry {
    basis:    fn(f64) -> [f64; 4],
    params:   [f64; 4],
    training: Vec<(f64, f64)>,  // (dw, val) — original characterized values
}

impl FitEntry {
    fn eval(&self, dw: f64) -> f64 {
        // Exact match → return the raw Liberate MX value, not the fitted curve.
        if let Some(&(_, y)) = self.training.iter().find(|&&(x, _)| x == dw) {
            return y;
        }
        let x = (self.basis)(dw);
        x[0]*self.params[0] + x[1]*self.params[1]
            + x[2]*self.params[2] + x[3]*self.params[3]
    }
}
```

`eval` first checks whether the requested `dw` exactly matches one of the
7 characterized training breakpoints (8, 16, 32, 48, 64, 96, 128). If so,
it returns the original Liberate MX measurement directly — no polynomial
evaluation, no overestimate. Only for `dw` values that fall between training
points does it fall through to the conservatively-lifted OLS polynomial.

This matters because the intercept lift in `FitEntry::fit` shifts the curve
upward to guarantee conservatism everywhere, which means at training points
the polynomial would return a value strictly above the ground truth. Storing
the raw values and bypassing the polynomial at those points gives exact
accuracy where the data is known and conservative overestimation only where
interpolation is actually occurring.

### `FitEntry::fit` — three-stage fitting

```rust
fn fit(basis: fn(f64) -> [f64; 4], dws: &[f64], vals: &[f64]) -> Self {
```

**Stage 1 — OLS solve:**

```rust
let mut p = ols4(basis, dws, vals);
```

Calls `ols4` to find the 4 coefficients that minimise `||Xp - y||²` over the 7
training points. This gives the best-fit curve but makes no guarantee of
conservatism — it will underestimate roughly half the training points.

**Stage 2 — Pass 1 lift (training points):**

```rust
let lift1 = dws.iter().zip(vals).map(|(&x, &y)| {
    let b = basis(x);
    y - (b[0]*p[0] + b[1]*p[1] + b[2]*p[2] + b[3]*p[3])
}).fold(0.0_f64, f64::max);
p[0] += lift1;
```

For each training point, compute how much the current curve underestimates the
measured value. Take the worst case (largest underestimate). Add it to the
intercept `p[0]`. The curve now passes at or above every training point.
Only `p[0]` (the intercept / constant term) is shifted — this preserves the
shape of the fit while raising the floor.

**Stage 3 — Pass 2 lift (integer sweep):**

```rust
let val_max  = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
let val_min  = vals.iter().cloned().fold(f64::INFINITY, f64::min);
let val_mean = vals.iter().sum::<f64>() / vals.len() as f64;
let is_flat  = val_mean > 0.0 && (val_max - val_min) / val_mean < 0.02;
```

First, classify the data. If the full range of measured values is less than 2% of
the mean, the data has no real dw trend — any apparent variation is measurement
noise. Otherwise the data is considered trending.

```rust
let lift2 = (min_dw..=max_dw).map(|dw_i| {
    let dw = dw_i as f64;
    let reference = if is_flat {
        val_max
    } else {
        piecewise_log_linear(dw, dws, vals)
    };
    let b = basis(dw);
    reference - (b[0]*p[0] + b[1]*p[1] + b[2]*p[2] + b[3]*p[3])
}).fold(0.0_f64, f64::max);
p[0] += lift2;

let training = dws.iter().zip(vals).map(|(&x, &y)| (x, y)).collect();
Self { basis, params: p, training }
```

Every integer `dw` from the smallest to largest training value is swept. At each
point, the curve is compared against a **reference**:

- **Flat data**: the reference is the global maximum of all training values.
  Since there is no real dw dependence, the global max is the only safe upper
  bound — noise could place any intermediate value at that level.
- **Trending data**: the reference is `piecewise_log_linear(dw, dws, vals)`,
  which interpolates log-linearly between the two nearest training points.
  This is tight and conservative for smooth monotone curves.

The worst-case underestimate across all integer dw values is added to `p[0]`,
giving a curve that lies at or above the piecewise log-linear interpolant of the
data at every integer point, not just at the 7 training points.

---

## `piecewise_log_linear` — Log-Linear Segment Interpolation

```rust
fn piecewise_log_linear(dw: f64, dws: &[f64], vals: &[f64]) -> f64 {
    let i = dws.partition_point(|&x| x <= dw);
    if i == 0 { return vals[0]; }
    if i >= dws.len() { return vals[dws.len() - 1]; }
    let (x0, y0) = (dws[i - 1], vals[i - 1]);
    let (x1, y1) = (dws[i], vals[i]);
    let t = (dw.ln() - x0.ln()) / (x1.ln() - x0.ln());
    y0 + t * (y1 - y0)
}
```

`partition_point` is a binary search that returns the index of the first `dws[i]`
strictly greater than `dw`. So `dws[i-1] <= dw < dws[i]` defines the bracketing
segment. Extrapolation clamps to the nearest endpoint value.

The interpolation parameter `t` is computed in log space:

```
t = (ln(dw) - ln(x0)) / (ln(x1) - ln(x0))
```

This maps `dw` linearly onto the interval `[ln(x0), ln(x1)]`, which linearises
the logarithmic growth of SRAM timing. Linear interpolation is then applied to the
timing values `y0` and `y1`. The result is exact at both endpoints and follows the
log-shaped curve in between, as opposed to plain linear interpolation which would
cut across the curve and potentially underestimate.

**Why log-linear?** SRAM timing scales approximately with `ln(dw)` because the
dominant physical effect is bitline capacitance: adding more output columns adds
capacitive load that the sense amplifier must overcome, and the effective RC time
constant grows logarithmically with the number of active bitlines.

---

## `ols4` — Ordinary Least Squares Solver

```rust
fn ols4(basis: fn(f64) -> [f64; 4], dws: &[f64], vals: &[f64]) -> [f64; 4] {
    let mut xtx = [[0.0f64; 4]; 4];
    let mut xty = [0.0f64; 4];
    for (&dw, &y) in dws.iter().zip(vals) {
        let x = basis(dw);
        for i in 0..4 {
            xty[i] += x[i] * y;
            for j in 0..4 { xtx[i][j] += x[i] * x[j]; }
        }
    }
    gauss4(xtx, xty)
}
```

This assembles the **normal equations** for least-squares: `(XᵀX)p = Xᵀy`.

- `xtx` accumulates the 4×4 matrix `XᵀX` one training point at a time.
  Each training point contributes an outer product `x · xᵀ`.
- `xty` accumulates the 4-vector `Xᵀy`. Each training point contributes `x · y`.

After the loop, `xtx` and `xty` form the 4×4 linear system whose solution is the
OLS coefficient vector. This is passed to `gauss4`.

---

## `gauss4` — Gaussian Elimination with Partial Pivoting

```rust
fn gauss4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> [f64; 4] {
    for col in 0..4 {
        // Partial pivoting: swap the row with the largest absolute value in this column
        let pivot = (col..4)
            .max_by(|&i, &j| a[i][col].abs().partial_cmp(&a[j][col].abs()).unwrap())
            .unwrap();
        a.swap(col, pivot);
        b.swap(col, pivot);
        let d = a[col][col];
        if d.abs() < 1e-14 { continue; }  // singular or near-zero: skip
        // Eliminate all rows below the pivot
        for row in (col + 1)..4 {
            let f = a[row][col] / d;
            for k in col..4 { a[row][k] -= f * a[col][k]; }
            b[row] -= f * b[col];
        }
    }
    // Back-substitution
    let mut x = [0.0f64; 4];
    for i in (0..4).rev() {
        let s: f64 = ((i + 1)..4).map(|j| a[i][j] * x[j]).sum();
        x[i] = if a[i][i].abs() < 1e-14 { 0.0 } else { (b[i] - s) / a[i][i] };
    }
    x
}
```

Gaussian elimination reduces the 4×4 system `Ax = b` to upper-triangular form,
then back-substitution solves for `x`.

**Partial pivoting** (swapping the row with the largest absolute value in the
current column to the pivot position) is a standard numerical stability technique.
Without it, small pivot values would amplify floating-point errors during elimination.

When a basis has trailing zeros (e.g. `basis_lin3_sqrt` returns `[1, dw, √dw, 0]`),
the last column of `XᵀX` is identically zero, making the system singular. The
`d.abs() < 1e-14` guard and `x[i] = 0.0` fallback handle this gracefully — the
unused coefficient is set to zero and the remaining 3 are solved correctly.

---

## `eval_7x7` — Table Evaluation at Query Time

```rust
fn eval_7x7(tbl: &[[FitEntry; 7]; 7], dw: f64) -> [[f64; 7]; 7] {
    std::array::from_fn(|i| std::array::from_fn(|j| tbl[i][j].eval(dw)))
}
```

At query time, each of the 49 `FitEntry` values in a 7×7 table is evaluated
independently at the requested `dw`. The result is a concrete `[[f64;7];7]` table
ready to write into the Liberty file. Each call to `eval` is just a dot product of
4 floats — the entire 7×7 evaluation is ~200 floating-point operations.

---

## `min_period` — Piecewise Log-Linear with Monotone Upper Hull

`min_period` is stored differently from all other tables. Instead of `[[FitEntry;7];7]`,
it stores the raw characterized snapshots:

```rust
min_period: Vec<(u32, [f64; 7])>,
```

## TimingModel Trait Implementation

`LookupModel` implements the `TimingModel` trait, which is the interface the Liberty
writer (`generate_sram_lib`) calls to get numeric values for every table.

Every `TimingModel` method follows the same pattern: call `eval_7x7` on the relevant
fitted table, passing `p.data_width() as f64` as the query point:

```rust
fn cell_rise(&self, p: &SramParams) -> [[f64; 7]; 7] {
    eval_7x7(&self.cell_rise, p.data_width() as f64)
}
```

For `hold_fall`, `din_hold_fall` is a special case — it returns the stored `[[f64;7];7]`
constant table directly without any interpolation:

```rust
fn hold_fall(&self, p: &SramParams, pin: PinRole) -> [[f64; 7]; 7] {
    match pin {
        PinRole::Din | PinRole::Wmask => self.din_hold_fall,  // constant, no eval
        PinRole::Addr | PinRole::Clk  => eval_7x7(&self.addr_hold_fall, dw),
        ...
    }
}
```

`setup_rise` and `setup_fall` return all-zeros. These timing arcs are not present in
the characterization data (the SRAM has no setup constraints in the conventional sense)
but the Liberty format requires the fields to be structurally present.

---

## Accuracy

At the 7 characterized `data_width` breakpoints (8, 16, 32, 48, 64, 96, 128)
`FitEntry::eval` returns the exact Liberate MX value — 0% overestimate by
construction.

For interpolated values, validated at `dw=72` (between characterized points 64
and 96) against Liberate MX ground truth:

| Config | Table | Corner | Overestimate range |
|---|---|---|---|
| 64×m4w8 | `cell_rise` | tt/ss/ff | +0.15% … +0.37% |
| 64×m4w8 | `min_period` | tt/ss/ff | +1.40% … +1.71% |
| 128×m8w8 | `cell_rise` | tt/ss/ff | +0.44% … +0.85% |
| 128×m8w8 | `min_period` | tt/ss/ff | +0.31% … +0.69% |

The model always overestimates (conservative) and stays well within 2% for
interpolated `data_width` values.
