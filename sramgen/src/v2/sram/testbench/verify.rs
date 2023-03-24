use std::collections::HashMap;

use substrate::verification::simulation::bits::{to_bit, BitSignal};
use substrate::verification::simulation::TranData;

use super::{Op, TbParams};
use anyhow::{anyhow, bail, Result};

pub(crate) fn verify_simulation(data: &TranData, tb: &TbParams) -> Result<()> {
    let mut state = HashMap::new();
    let data_bits_per_wmask = tb.sram.data_width / tb.sram.wmask_width;

    // Clock cycle counter
    // Initialized to 1 instead of 0,
    // since nothing happens on the first cycle of our testbench.
    let mut cycle = 1;

    for op in tb.ops.iter() {
        cycle += 1;
        match op {
            Op::Read { addr } => {
                let expected: &BitSignal = state
                    .get(addr)
                    .ok_or_else(|| anyhow!("Attempted to read an uninitialized address."))?;

                let t = cycle as f64 * tb.clk_period;
                let idx = data
                    .time
                    .idx_before_sorted(t)
                    .ok_or_else(|| anyhow!("Time {} was out of simulation range", t))?;
                for i in 0..tb.sram.data_width {
                    let name = format!("{}[{}]", "Xdut.dout", i);
                    let rx_bit = data
                        .data
                        .get(&name)
                        .ok_or_else(|| anyhow!("Unable to find signal {}", &name))?
                        .get(idx)
                        .ok_or_else(|| {
                            anyhow!("Index {} was out of range for signal {}", idx, &name)
                        })?;
                    let rx_bit = to_bit(rx_bit, tb.vdd)?;
                    let ex_bit = expected.bit(i);
                    if rx_bit != ex_bit {
                        bail!(
                            "reading addr {}: expected bit {} to be {}; got {} at clock cycle {} (time {}, index {})",
                            addr, i, ex_bit, rx_bit, cycle-1, t, idx
                        );
                    }
                }
            }
            Op::Write { addr, data } => {
                state.insert(addr.to_owned(), data.to_owned());
            }
            Op::WriteMasked { addr, data, mask } => {
                // If performing a masked write, that address should already have been initialized.
                let entry = state.get_mut(addr).ok_or_else(|| {
                    anyhow!("Attempted to perform a masked write at an uninitialized address")
                })?;
                for (i, bit) in mask.bits().enumerate() {
                    if bit {
                        for j in i * data_bits_per_wmask..(i + 1) * data_bits_per_wmask {
                            entry.assign(j, data.bit(j));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
