use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use itertools::izip;
use substrate::verification::simulation::bits::{to_bit, BitSignal};
use substrate::verification::simulation::waveform::{TimeWaveform, Transition, Waveform};
use substrate::verification::simulation::TranData;

use crate::blocks::sram::SramPhysicalDesign;

use super::{Op, TbParams, TbSignals};
use anyhow::{anyhow, bail, Result};

/// Reports relevant behavior of internal signals for diagnostic purposes.
pub fn write_internal_rpt(
    work_dir: impl AsRef<Path>,
    data: &TranData,
    tb: &TbParams,
) -> Result<()> {
    let rpt_path = work_dir.as_ref().join("internal.rpt");
    let mut rpt = File::create(rpt_path)?;
    let low_threshold = 0.2 * tb.vdd;
    let high_threshold = 0.8 * tb.vdd;

    // Assert that only the appropriate wordline goes high during reads and writes
    // and that no wordline goes high when no operation is occuring.
    //
    // Also assert that write driver enable pulse (after NAND with write mask)
    // overlaps with wordline pulse for at least 50 ps.
    //
    // Assert that PC_B, SENSE_EN, and WRITE_DRIVER_EN remain low during a no-op.
    writeln!(rpt, "OPS")?;
    writeln!(rpt, "==========================")?;
    let wl = (0..tb.sram.rows())
        .map(|i| {
            data.waveform(&tb.sram_signal_path(TbSignals::WlEnd(i)))
                .ok_or_else(|| anyhow!("Unable to find signal wl"))
        })
        .collect::<Result<Vec<_>>>()?;
    let we_i = (0..tb.sram.wmask_width())
        .map(|i| {
            data.waveform(&tb.sram_signal_path(TbSignals::WeI(i)))
                .ok_or_else(|| anyhow!("Unable to find signal we_i"))
        })
        .collect::<Result<Vec<_>>>()?;
    let we_ib = (0..tb.sram.wmask_width())
        .map(|i| {
            data.waveform(&tb.sram_signal_path(TbSignals::WeIb(i)))
                .ok_or_else(|| anyhow!("Unable to find signal we_ib"))
        })
        .collect::<Result<Vec<_>>>()?;
    let pc_b = data
        .waveform(&tb.sram_signal_path(TbSignals::PcBEnd))
        .ok_or_else(|| anyhow!("Unable to find signal pc_b"))?;
    let saen = data
        .waveform(&tb.sram_signal_path(TbSignals::SenseEnEnd))
        .ok_or_else(|| anyhow!("Unable to find signal sense_en"))?;
    let wrdrven = data
        .waveform(&tb.sram_signal_path(TbSignals::WriteDriverEnEnd))
        .ok_or_else(|| anyhow!("Unable to find signal write_driver_en"))?;
    let mut cycle = 0;
    let [mut wl_trans, mut we_i_trans, mut we_ib_trans] = [&wl, &we_i, &we_ib].map(|wfs| {
        wfs.iter()
            .map(|wf| {
                wf.transitions(low_threshold, high_threshold)
                    .collect::<VecDeque<_>>()
            })
            .collect::<Vec<_>>()
    });
    let [mut pc_b_trans, mut saen_trans, mut wrdrven_trans] = [&pc_b, &saen, &wrdrven].map(|wf| {
        wf.transitions(low_threshold, high_threshold)
            .collect::<VecDeque<_>>()
    });
    for op in tb.ops.iter() {
        cycle += 1;
        let t = cycle as f64 * tb.clk_period;
        let idx = data
            .time
            .idx_before_sorted(t)
            .ok_or_else(|| anyhow!("Time {} was out of simulation range", t))?;
        let mut active_wls = Vec::new();
        for i in 0..tb.sram.rows() {
            if wl[i].get(idx).unwrap().x() > high_threshold {
                writeln!(
                    rpt,
                    "ERROR: wordline {i} is high at beginning of cycle {cycle}"
                )?;
            }
            while let Some(next) = wl_trans[i].front() {
                if next.center_time() < t {
                    wl_trans[i].pop_front();
                } else {
                    break;
                }
            }
            if let Some(next) = wl_trans[i].front() {
                if next.center_time() < (cycle + 1) as f64 * tb.clk_period {
                    active_wls.push(i);
                }
            }
        }
        for i in 0..tb.sram.wmask_width() {
            for we_trans in [&mut we_i_trans[i], &mut we_ib_trans[i]] {
                while let Some(next) = we_trans.front() {
                    if next.center_time() < t {
                        we_trans.pop_front();
                    } else {
                        break;
                    }
                }
            }
        }
        for (signal, trans) in [
            ("pc_b", &mut pc_b_trans),
            ("saen", &mut saen_trans),
            ("wrdrven", &mut wrdrven_trans),
        ] {
            while let Some(next) = trans.front() {
                if next.center_time() < t {
                    trans.pop_front();
                } else {
                    break;
                }
            }
            if let (Some(next), Op::None) = (trans.front(), op) {
                if next.center_time() < (cycle + 1) as f64 * tb.clk_period {
                    writeln!(rpt, "ERROR: transition in {signal} occurs during no-op")?;
                }
            }
        }

        match op {
            Op::Read { addr } | Op::Write { addr, .. } | Op::WriteMasked { addr, .. } => {
                if active_wls.len() > 1 {
                    writeln!(
                        rpt,
                        "ERROR: multiple active wordlines ({active_wls:?}) during operation on cycle {cycle}"
                    )?;
                } else if active_wls.is_empty() {
                    writeln!(rpt, "ERROR: no active wordlines ({active_wls:?}) during operation on cycle {cycle}")?;
                } else if BitSignal::from_u64(active_wls[0] as u64, tb.sram.row_bits())
                    != BitSignal::from(addr.inner()[tb.sram.col_select_bits()..].to_bitvec())
                {
                    writeln!(rpt, "ERROR: active wordline {} does not correspond to addr {addr} during cycle {cycle}", active_wls[0])?;
                }
            }
            _ => {
                if !active_wls.is_empty() {
                    writeln!(
                        rpt,
                        "ERROR: active wordlines ({active_wls:?}) during cycle {cycle} while no operation is occuring"
                    )?;
                }
            }
        }
        if matches!(op, Op::Write { .. }) {
            let check_overlap = |we_trans: &VecDeque<Transition>| -> Option<_> {
                let active_wl = active_wls.first()?;
                let wl_start = wl_trans[*active_wl].get(0)?.center_time();
                let wl_end = wl_trans[*active_wl].get(1)?.center_time();
                let wrdrven_start = we_trans.get(0)?.center_time();
                let wrdrven_end = we_trans.get(1)?.center_time();
                let overlap_start = if wl_start > wrdrven_start {
                    wl_start
                } else {
                    wrdrven_start
                };
                let overlap_end = if wl_end < wrdrven_end {
                    wl_end
                } else {
                    wrdrven_end
                };
                let overlap = (overlap_end - overlap_start) * 1e12;
                Some((overlap, active_wl, overlap_start))
            };
            for i in 0..tb.sram.wmask_width() {
                for we_trans in [&we_i_trans[i], &we_ib_trans[i]] {
                    if let Some((overlap, active_wl, overlap_start)) = check_overlap(we_trans) {
                        if overlap < 50. {
                            writeln!(rpt, "WARNING: overlap between we_i[{i}] and wordline {active_wl} is less than 50 ps at t = {} ps", overlap_start * 1e12)?;
                        }
                        writeln!(rpt, "Overlap of {overlap} ps between we_i[{i}] and wordline {active_wl} at t = {} ps", overlap_start * 1e12)?;
                    }
                }
            }
        }
    }
    writeln!(rpt)?;

    // Assert that decoder replica matches row decoder delay.
    let decrepstart = data
        .waveform(&tb.sram_signal_path(TbSignals::Decrepstart))
        .ok_or_else(|| anyhow!("Unable to find signal decrepstart"))?;
    let decrepend = data
        .waveform(&tb.sram_signal_path(TbSignals::Decrepend))
        .ok_or_else(|| anyhow!("Unable to find signal decrepend"))?;
    let wlen = data
        .waveform(&tb.sram_signal_path(TbSignals::Wlen))
        .ok_or_else(|| anyhow!("Unable to find signal wlen"))?;
    let wl_max = wl
        .iter()
        .fold(None, |a: Option<Waveform>, b| {
            let mut wf_new = Waveform::new();
            if let Some(wf) = a {
                for (a, b) in b.values().zip(wf.values()) {
                    wf_new.push(a.t(), if a.x() < b.x() { b.x() } else { a.x() });
                }
            } else {
                for val in b.values() {
                    wf_new.push(val.t(), val.x());
                }
            }
            Some(wf_new)
        })
        .unwrap();

    writeln!(rpt, "DECODER REPLICA")?;
    writeln!(rpt, "==========================")?;
    for (decrepstart_trans, decrepend_trans, wlen_trans, wl_trans) in izip!(
        decrepstart.transitions(low_threshold, high_threshold),
        decrepend.transitions(low_threshold, high_threshold),
        wlen.transitions(low_threshold, high_threshold),
        wl_max.transitions(low_threshold, high_threshold)
    ) {
        let decrep_delay = decrepend_trans.center_time() - decrepstart_trans.center_time();
        let decoder_delay = wl_trans.center_time() - wlen_trans.center_time();

        writeln!(
            rpt,
            "Transition @ {} ps: {:?}",
            decrepstart_trans.center_time() * 1e12,
            decrepstart_trans.dir()
        )?;
        writeln!(rpt, "Replica delay: {} ps", decrep_delay * 1e12)?;
        writeln!(rpt, "Decoder delay: {} ps", decoder_delay * 1e12)?;
        writeln!(rpt)?;
    }

    // Assert that precharge turns off before wordline or write driver is enabled and
    // turns on only after wordline and write driver are disabled.
    // Also asserts that precharge turns on after sense amp enable.
    writeln!(rpt, "PRECHARGE")?;
    writeln!(rpt, "==========================")?;

    for trans in pc_b.transitions(low_threshold, high_threshold) {
        for idx in [trans.start_idx(), trans.end_idx()] {
            for i in 0..wl.len() {
                if wl[i].get(idx).unwrap().x() > high_threshold {
                    writeln!(
                        rpt,
                        "WARNING: wordline {i} high during pc_b transition at t={} ps",
                        trans.center_time() * 1e12
                    )?;
                }
            }
            for i in 0..tb.sram.wmask_width() {
                if we_i[i].get(idx).unwrap().x() > high_threshold {
                    writeln!(
                        rpt,
                        "WARNING: we_i[{i}] high during pc_b transition at t={} ps",
                        trans.center_time() * 1e12
                    )?;
                }
                if we_ib[i].get(idx).unwrap().x() < low_threshold {
                    writeln!(
                        rpt,
                        "WARNING: we_ib[{i}] low during pc_b transition at t={} ps",
                        trans.center_time() * 1e12
                    )?;
                }
            }
        }
    }

    for (i, trans) in wl
        .iter()
        .map(|wf| wf.transitions(low_threshold, high_threshold))
        .enumerate()
    {
        for trans in trans {
            for idx in [trans.start_idx(), trans.end_idx()] {
                if pc_b.get(idx).unwrap().x() < low_threshold {
                    writeln!(
                        rpt,
                        "WARNING: pc_b low during wl[{i}] transition at t={} ps ",
                        trans.center_time() * 1e12
                    )?;
                }
            }
        }
    }
    for (i, trans) in we_i
        .iter()
        .map(|wf| wf.transitions(low_threshold, high_threshold))
        .enumerate()
    {
        for trans in trans {
            for idx in [trans.start_idx(), trans.end_idx()] {
                if pc_b.get(idx).unwrap().x() < low_threshold {
                    writeln!(
                        rpt,
                        "WARNING: pc_b low during we_i[{i}] transition at t={} ps ",
                        trans.center_time() * 1e12
                    )?;
                }
            }
        }
    }
    for (i, trans) in we_ib
        .iter()
        .map(|wf| wf.transitions(low_threshold, high_threshold))
        .enumerate()
    {
        for trans in trans {
            for idx in [trans.start_idx(), trans.end_idx()] {
                if pc_b.get(idx).unwrap().x() < low_threshold {
                    writeln!(
                        rpt,
                        "WARNING: pc_b low during we_ib[{i}] transition at t={} ps ",
                        trans.center_time() * 1e12
                    )?;
                }
            }
        }
    }
    for trans in saen.transitions(low_threshold, high_threshold) {
        for idx in [trans.start_idx(), trans.end_idx()] {
            if trans.dir().is_rising() && pc_b.get(idx).unwrap().x() < low_threshold {
                writeln!(
                    rpt,
                    "WARNING: pc_b low during rising sense_en transition at t={} ps ",
                    trans.center_time() * 1e12
                )?;
            }
        }
    }
    writeln!(rpt)?;

    // Assert that the sense amp turns on after there is a 150 mV differential in bitlines.
    writeln!(rpt, "SENSE AMP ENABLE")?;
    writeln!(rpt, "==========================")?;
    let bl = (0..tb.sram.cols())
        .map(|i| {
            data.waveform(&tb.sram_signal_path(TbSignals::Bl(i)))
                .ok_or_else(|| anyhow!("Unable to find signal bl"))
        })
        .collect::<Result<Vec<_>>>()?;
    let br = (0..tb.sram.cols())
        .map(|i| {
            data.waveform(&tb.sram_signal_path(TbSignals::Br(i)))
                .ok_or_else(|| anyhow!("Unable to find signal br"))
        })
        .collect::<Result<Vec<_>>>()?;
    for trans in saen.transitions(low_threshold, high_threshold) {
        if trans.dir().is_rising() {
            let idx = trans.start_idx();
            let mut min_diff = f64::MAX;
            for (i, (bl, br)) in bl.iter().zip(br.iter()).enumerate() {
                let diff = (bl.get(idx).unwrap().x() - br.get(idx).unwrap().x()).abs();
                if diff < 0.15 {
                    writeln!(rpt, "WARNING: bitline {i} differential is less than 150 mV during sense amp read at t = {} ps", trans.start_time() * 1e12)?;
                }
                if diff < min_diff {
                    min_diff = diff;
                }
            }
            writeln!(rpt, "Transition @ {} ps", trans.center_time() * 1e-12)?;
            writeln!(rpt, "Minimum differential: {min_diff} V")?;
            writeln!(rpt)?;
        }
    }

    Ok(())
}

pub fn verify_simulation(work_dir: impl AsRef<Path>, data: &TranData, tb: &TbParams) -> Result<()> {
    let mut state = HashMap::new();
    let data_bits_per_wmask = tb.sram.data_width / tb.sram.wmask_width();

    if let Err(e) = write_internal_rpt(work_dir, data, tb) {
        println!("ERROR: Failed to write internal report ({:?})", e);
    }

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
                    let name = format!("{}[{}]", "dout", i);
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
            _ => {}
        }
    }

    Ok(())
}
