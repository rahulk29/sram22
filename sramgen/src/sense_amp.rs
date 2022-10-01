use std::collections::HashMap;

use vlsir::circuit::Instance;
use vlsir::Module;

use crate::tech::sramgen_sp_sense_amp_ref;
use crate::utils::conns::conn_slice;
use crate::utils::{bus, port_inout, port_input, port_output, sig_conn, signal};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SenseAmpArrayParams {
    pub name: String,
    pub width: i64,
}

pub fn sense_amp_array(params: SenseAmpArrayParams) -> Module {
    assert!(params.width > 0);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");
    let bl = bus("bl", params.width);
    let br = bus("br", params.width);
    let data = bus("data", params.width);
    let _data_b = bus("data_b", params.width);

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&clk),
        port_input(&bl),
        port_input(&br),
        port_output(&data),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..params.width {
        let mut connections = HashMap::new();
        connections.insert("clk".to_string(), sig_conn(&clk));
        connections.insert("inn".to_string(), conn_slice("br", i, i));
        connections.insert("inp".to_string(), conn_slice("bl", i, i));
        connections.insert("outp".to_string(), conn_slice("data", i, i));
        connections.insert("outn".to_string(), conn_slice("data_b", i, i));
        connections.insert("VDD".to_string(), sig_conn(&vdd));
        connections.insert("VSS".to_string(), sig_conn(&vss));

        m.instances.push(Instance {
            name: format!("sense_amp_{}", i),
            module: Some(sramgen_sp_sense_amp_ref()),
            parameters: HashMap::new(),
            connections,
        });
    }

    m
}

#[cfg(test)]
mod tests {
    use vlsir::circuit::Package;

    use crate::save_bin;
    use crate::tech::all_external_modules;

    use super::*;

    #[test]
    fn test_netlist_sense_amp_array() -> Result<(), Box<dyn std::error::Error>> {
        let sense_amps = sense_amp_array(SenseAmpArrayParams {
            name: "sense_amp_array".to_string(),
            width: 64 / 4,
        });
        let ext_modules = all_external_modules();
        let pkg = Package {
            domain: "sramgen_sense_amp_array".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![sense_amps],
            ext_modules,
        };

        save_bin("sense_amp_array", pkg)?;

        Ok(())
    }
}
