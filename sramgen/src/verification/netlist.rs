use crate::Result;
use std::fmt::Write;
use super::waveform::Waveform;
use super::{TbParams, TbWaveforms};

pub fn generate_netlist(params: &TbParams, waveforms: &TbWaveforms) -> crate::Result<String> {
    let mut out = String::new();
    let gnd_net = "vss";

    let TbWaveforms { addr, din, clk, we, wmask } = waveforms;

    write_pwl(&mut out, &params.clk_port, gnd_net, clk)?;
    write_pwl(&mut out, &params.write_enable_port, gnd_net, we)?;
    write_pwl_bus(&mut out, &params.addr_port, gnd_net, &addr)?;
    write_pwl_bus(&mut out, &params.data_in_port, gnd_net, &din)?;

    if !wmask.is_empty() {
        write_pwl_bus(&mut out, params.wmask_port.as_ref().unwrap(), gnd_net, &addr)?;
    }

    Ok(out)
}

fn write_pwl_bus(out: &mut String, port: &str, gnd_net: &str, waveforms: &[Waveform]) -> Result<()> {
    for (i, wav) in waveforms.iter().enumerate() {
        write_pwl(out, &format!("{port}[{i}]"), gnd_net, wav)?;
    }
    Ok(())
}

fn write_pwl(out: &mut String, net: &str, gnd_net: &str, waveform: &Waveform) -> Result<()> {
    writeln!(out, "V{net} {net} {gnd_net} pwl(")?;
    for (t, x) in waveform.values() {
        writeln!(out, "+ {t} {x}")?;
    }
    writeln!(out, "+ )")?;
    Ok(())
}
