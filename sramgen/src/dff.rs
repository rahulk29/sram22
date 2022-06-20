use std::collections::HashMap;

use pdkprims::{config::Int, mos::MosType};

use vlsir::circuit::Instance;
use vlsir::{circuit::Module, reference::To, Reference};

use crate::tech::openram_dff_ref;
use crate::utils::port_output;
use crate::{
    mos::Mosfet,
    utils::{bus, conns::conn_slice, port_inout, port_input, sig_conn, signal},
};

pub struct DffArrayParams {
    pub name: String,
    pub width: usize,
}

pub fn dff_array(params: DffArrayParams) -> Vec<Module> {
    let width = params.width as i64;

    assert!(width > 0);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");

    let d = bus("d", width);
    let q = bus("q", width);
    let q_b = bus("q_b", width);

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&clk),
        port_input(&d),
        port_output(&q),
        port_output(&q_b),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..width {
        let mut connections = HashMap::new();
        connections.insert("VDD".to_string(), sig_conn(&vdd));
        connections.insert("GND".to_string(), sig_conn(&vss));
        connections.insert("CLK".to_string(), sig_conn(&clk));
        connections.insert("D".to_string(), conn_slice("d", i, i));
        connections.insert("Q".to_string(), conn_slice("q", i, i));
        connections.insert("Q_N".to_string(), conn_slice("q_b", i, i));

        m.instances.push(Instance {
            name: format!("dff_{}", i),
            module: Some(openram_dff_ref()),
            parameters: HashMap::new(),
            connections,
        });
    }

    vec![m]
}

#[cfg(test)]
mod tests {
    use vlsir::circuit::Package;

    use crate::{save_bin, tech::all_external_modules, utils::save_modules};

    use super::*;

    #[test]
    fn test_sky130_dff_array() -> Result<(), Box<dyn std::error::Error>> {
        let dffs = dff_array(DffArrayParams {
            width: 16,
            name: "dff_array".to_string(),
        });

        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "sramgen_dff_array".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: dffs,
            ext_modules,
        };

        save_bin("dff_array", pkg)?;

        Ok(())
    }
}
