use super::waveform::Waveform;
use super::{TbParams, TbWaveforms};
use crate::verification::PortOrder;
use crate::Result;
use std::fmt::Write;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TbNetlistParams<'a> {
    pub tb: &'a TbParams,
    pub waveforms: &'a TbWaveforms,
}

pub fn generate_netlist(params: TbNetlistParams) -> crate::Result<String> {
    let TbNetlistParams { tb, waveforms } = params;
    let mut out = String::new();

    let TbWaveforms {
        addr,
        din,
        clk,
        we,
        wmask,
    } = waveforms;

    writeln!(
        &mut out,
        "* SRAM22 generated testbench for {}",
        tb.sram_name
    )?;
    writeln!(&mut out, ".param t_end={}", clk.last_t().unwrap())?;
    writeln!(&mut out, ".tran 1.00e-12 't_end'")?;
    write_spacer(&mut out)?;

    for include in tb.includes.iter() {
        writeln!(&mut out, ".include {}", include)?;
    }
    for include in tb.source_paths.iter() {
        writeln!(&mut out, ".include {:?}", include)?;
    }
    write_spacer(&mut out)?;

    write_dut(&mut out, tb)?;

    let gnd_net = &tb.gnd_port;

    writeln!(&mut out, "Vvdd {} {} {}", tb.pwr_port, gnd_net, tb.vdd)?;
    writeln!(&mut out, "Vvss {} 0 0", gnd_net)?;
    write_spacer(&mut out)?;
    writeln!(&mut out, ".option parhier=local redefinedparams=ignore")?;
    writeln!(&mut out, "simulator lang=spectre")?;
    writeln!(&mut out, "altos_op1 options global_param_override=ignore")?;
    writeln!(&mut out, "simulator lang=spice")?;
    write_spacer(&mut out)?;

    write_pwl(&mut out, &tb.clk_port, gnd_net, clk)?;
    write_pwl(&mut out, &tb.write_enable_port, gnd_net, we)?;
    write_pwl_bus(&mut out, &tb.addr_port, gnd_net, addr)?;
    write_pwl_bus(&mut out, &tb.data_in_port, gnd_net, din)?;
    if !wmask.is_empty() {
        write_pwl_bus(&mut out, tb.wmask_port.as_ref().unwrap(), gnd_net, addr)?;
    }

    write_spacer(&mut out)?;
    write_cap_loads(
        &mut out,
        &tb.data_out_port,
        tb.data_width,
        gnd_net,
        tb.c_load,
    )?;
    write_spacer(&mut out)?;
    write_probe(&mut out, &tb.clk_port)?;
    write_probe(&mut out, &tb.write_enable_port)?;
    if let Some(ref wmask_port) = tb.wmask_port {
        write_probes(&mut out, wmask_port, tb.wmask_groups)?;
    }
    write_probes(&mut out, &tb.data_out_port, tb.data_width)?;
    write_probes(&mut out, &tb.data_in_port, tb.data_width)?;
    write_probes(&mut out, &tb.addr_port, tb.addr_width)?;

    write_spacer(&mut out)?;
    writeln!(&mut out, ".temp 25")?;
    writeln!(&mut out, ".end")?;
    write_spacer(&mut out)?;

    Ok(out)
}

fn write_pwl_bus(
    out: &mut String,
    port: &str,
    gnd_net: &str,
    waveforms: &[Waveform],
) -> Result<()> {
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

fn write_dut(out: &mut String, tb: &TbParams) -> Result<()> {
    writeln!(out, "XDUT0 ")?;
    for (port_class, order) in tb.ports.iter().copied() {
        let port_name = tb.port_name(port_class);
        if port_class.is_bus() {
            let width = tb.port_width(port_class);
            assert!(width > 1);
            match order {
                PortOrder::LsbFirst => {
                    for i in 0..width {
                        writeln!(out, "+ {port_name}[{i}]")?;
                    }
                }
                PortOrder::MsbFirst => {
                    for i in (0..width).rev() {
                        writeln!(out, "+ {port_name}[{i}]")?;
                    }
                }
            }
        } else {
            writeln!(out, "+ {port_name} ")?;
        }
    }
    writeln!(out, "+ {}", tb.sram_name)?;
    Ok(())
}

fn write_cap_loads(
    out: &mut String,
    port: &str,
    width: usize,
    gnd_net: &str,
    c_load: f64,
) -> Result<()> {
    writeln!(out, "* LOAD CAPACITORS")?;
    for i in 0..width {
        writeln!(out, "C{port}[{i}] {port}[{i}] {gnd_net} {c_load}")?;
    }
    Ok(())
}

fn write_probes(out: &mut String, port: &str, width: usize) -> Result<()> {
    writeln!(out, "* PROBES FOR {port}")?;
    for i in 0..width {
        writeln!(out, ".probe v({port}[{i}])")?;
    }
    Ok(())
}

fn write_probe(out: &mut String, net: &str) -> Result<()> {
    writeln!(out, "* PROBE FOR {net}")?;
    writeln!(out, ".probe v({net})")?;
    Ok(())
}

fn write_spacer(out: &mut String) -> Result<()> {
    writeln!(out, "\n")?;
    Ok(())
}

pub fn write_netlist(path: impl AsRef<Path>, netlist: &str) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, netlist)?;

    Ok(())
}
