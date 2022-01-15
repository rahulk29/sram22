use crate::backend::NetlistBackend;
use crate::cells::gates::nand3;
use crate::error::Result;

pub struct PredecoderOptions<'a> {
    pub(crate) power_net: &'a str,
    pub(crate) gnd_net: &'a str,
}

pub fn netlist(b: &mut dyn NetlistBackend, opts: PredecoderOptions) -> Result<()> {
    // b.subcircuit("predecode_3_8")?;
    for i in [0, 1] {
        let a2 = net_name("A2", i);
        for j in [0, 1] {
            let a1 = net_name("A1", j);
            for k in [0, 1] {
                let a0 = net_name("A0", k);
                let nand_name = format!("NAND{}{}{}", i, j, k);
                let out_name = format!("PDEC{}{}{}", i, j, k);
                nand3::netlist(
                    b,
                    &nand_name,
                    &a2,
                    &a1,
                    &a0,
                    &out_name,
                    opts.gnd_net,
                    opts.power_net,
                )?;
            }
        }
    }
    // b.end_subcircuit()?;

    Ok(())
}

fn net_name(base: &str, x: i32) -> String {
    match x {
        0 => format!("{}", base),
        1 => format!("{}b", base),
        _ => unreachable!(),
    }
}
