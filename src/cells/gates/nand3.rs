use crate::backend::NetlistBackend;
use crate::error::Result;

pub fn netlist(
    backend: &mut dyn NetlistBackend,
    name: &str,
    a: &str,
    b: &str,
    c: &str,
    y: &str,
    gnd: &str,
    vdd: &str,
) -> Result<()> {
    let n1 = backend.temp_net();
    let n2 = backend.temp_net();

    backend.instance(
        &format!("{}_MNC", name),
        &[&n2, c, gnd, gnd],
        "sky130_fd_pr__nfet_01v8",
        &["w=1e+06u", "l=150000u"],
    )?;
    backend.instance(
        &format!("{}_MNB", name),
        &[&n1, b, &n2, gnd],
        "sky130_fd_pr__nfet_01v8",
        &["w=1e+06u", "l=150000u"],
    )?;
    backend.instance(
        &format!("{}_MNA", name),
        &[y, a, &n1, gnd],
        "sky130_fd_pr__nfet_01v8",
        &["w=1e+06u", "l=150000u"],
    )?;

    backend.instance(
        &format!("{}_MPA", name),
        &[y, a, vdd, vdd],
        "sky130_fd_pr__pfet_01v8",
        &["w=1e+06u", "l=150000u"],
    )?;
    backend.instance(
        &format!("{}_MPB", name),
        &[y, b, vdd, vdd],
        "sky130_fd_pr__pfet_01v8",
        &["w=1e+06u", "l=150000u"],
    )?;
    backend.instance(
        &format!("{}_MPC", name),
        &[y, c, vdd, vdd],
        "sky130_fd_pr__pfet_01v8",
        &["w=1e+06u", "l=150000u"],
    )?;
    Ok(())
}
