use std::collections::HashMap;

use pdkprims::config::Int;
use pdkprims::mos::MosType;

use vlsir::circuit::Module;
use vlsir::reference::To;
use vlsir::Reference;

use crate::gate::{and2, AndParams};
use crate::mos::Mosfet;
use crate::utils::conns::conn_slice;
use crate::utils::{bus, port_inout, port_input, port_output, sig_conn, signal};

pub struct WriteMaskControlParams {
    pub name: String,
    pub width: i64,
    pub and_params: AndParams,
}

pub fn write_mask_control(params: WriteMaskControlParams) -> Vec<Module> {
    assert!(params.width > 0);

    let mut and = and2(params.and_params.clone());

    let vdd = signal("vdd");
    let vss = signal("vss");
    let wr_en = signal("wr_en");
    let sel = bus("sel", params.width);
    let write_driver_en = bus("write_driver_en", params.width);

    let ports = vec![
        port_input(&wr_en),
        port_input(&sel),
        port_output(&write_driver_en),
        port_inout(&vdd),
        port_inout(&vss),
    ];

    let mut m = Module {
        name: params.name.clone(),
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..params.width {
        let conns = [
            ("vdd", sig_conn(&vdd)),
            ("vss", sig_conn(&vss)),
            ("a", conn_slice("sel", i, i)),
            ("b", sig_conn(&wr_en)),
            ("y", conn_slice("write_driver_en", i, i)),
        ];
        m.instances.push(vlsir::circuit::Instance {
            name: format!("and2_{}", i),
            module: Some(Reference {
                to: Some(To::Local(params.and_params.name.clone())),
            }),
            parameters: HashMap::new(),
            connections: crate::utils::conn_map(conns.into()),
        });
    }

    let mut modules = Vec::new();
    modules.append(&mut and);
    modules.push(m);
    modules
}

#[cfg(test)]
mod tests {
    use crate::gate::Size;
    use crate::utils::save_modules;

    use super::*;

    #[test]
    fn test_netlist_write_mask_control() -> Result<(), Box<dyn std::error::Error>> {
        let modules = write_mask_control(WriteMaskControlParams {
            name: "write_mask_control".to_string(),
            width: 2,
            and_params: AndParams {
                name: "write_mask_control_and2".to_string(),
                nand_size: Size {
                    nmos_width: 1_200,
                    pmos_width: 1_800,
                },
                inv_size: Size {
                    nmos_width: 1_200,
                    pmos_width: 1_800,
                },
                length: 150,
            },
        });
        save_modules("write_mask_control", modules)?;
        Ok(())
    }
}
