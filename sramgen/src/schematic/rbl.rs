use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::schematic::gate::{inv, GateParams, Size};
use crate::schematic::precharge::{precharge, PrechargeParams};
use crate::tech::sram_sp_replica_cell_ref;
use crate::utils::{
    conn_map, local_reference, port_inout, port_input, port_output, sig_conn, signal,
};

#[derive(Debug, Clone)]
pub struct ReplicaBitcellColumnParams {
    pub name: String,
    pub num_active_cells: i64,
    pub height: i64,
}

#[derive(Debug, Clone)]
pub struct ReplicaColumnParams {
    pub name: String,
    pub bitcell_params: ReplicaBitcellColumnParams,
}

pub fn replica_bitcell_column(params: ReplicaBitcellColumnParams) -> Vec<Module> {
    assert_eq!(params.num_active_cells % 2, 0);
    assert_eq!(params.height % 2, 0);
    assert!(params.num_active_cells <= params.height);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let rwl = signal("rwl");
    let rbl = signal("rbl");
    let rbr = signal("rbr");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&rwl),
        port_inout(&rbl),
        port_inout(&rbr),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    for i in 0..params.height {
        let mut conns = HashMap::new();
        conns.insert("BL", sig_conn(&rbl));
        conns.insert("BR", sig_conn(&rbr));
        conns.insert("VPWR", sig_conn(&vdd));
        conns.insert("VPB", sig_conn(&vdd));
        conns.insert("VGND", sig_conn(&vss));
        conns.insert("VNB", sig_conn(&vss));

        let wl_conn = if i < params.num_active_cells {
            sig_conn(&rwl)
        } else {
            sig_conn(&vss)
        };

        conns.insert("WL", wl_conn);
        m.instances.push(Instance {
            name: format!("replica_bitcell_{}", i),
            parameters: HashMap::new(),
            module: Some(sram_sp_replica_cell_ref()),
            connections: conn_map(conns),
        });
    }

    vec![m]
}

pub fn replica_column(params: ReplicaColumnParams) -> Vec<Module> {
    let mut bitcells = replica_bitcell_column(params.bitcell_params.clone());
    let pc_params = PrechargeParams {
        name: "replica_precharge".to_string(),
        length: 150,
        pull_up_width: 2_400,
        equalizer_width: 1_200,
    };
    let precharge = precharge(pc_params.clone());

    let inv_params = GateParams {
        name: "replica_bl_inv".to_string(),
        length: 150,
        size: Size {
            nmos_width: 1_200,
            pmos_width: 2_000,
        },
    };
    let inv = inv(inv_params.clone());

    let vdd = signal("vdd");
    let vss = signal("vss");
    let rwl = signal("rwl");
    let rbl = signal("rbl");
    let rbr = signal("rbr");
    let pc_b = signal("pc_b");
    let sae_i = signal("sae_i");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&rwl),
        port_input(&pc_b),
        port_output(&sae_i),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    // Replica bitcells
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("rwl", sig_conn(&rwl));
    conns.insert("rbl", sig_conn(&rbl));
    conns.insert("rbr", sig_conn(&rbr));
    m.instances.push(Instance {
        name: "replica_bitcells".to_string(),
        parameters: HashMap::new(),
        module: local_reference(&params.bitcell_params.name),
        connections: conn_map(conns),
    });

    // Precharge
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("en_b", sig_conn(&pc_b));
    conns.insert("bl", sig_conn(&rbl));
    conns.insert("br", sig_conn(&rbr));
    m.instances.push(Instance {
        name: "replica_precharge".to_string(),
        parameters: HashMap::new(),
        module: local_reference(&pc_params.name),
        connections: conn_map(conns),
    });

    // Inverter
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("gnd", sig_conn(&vss));
    conns.insert("din", sig_conn(&rbl));
    conns.insert("din_b", sig_conn(&sae_i));
    m.instances.push(Instance {
        name: "replica_bl_inv".to_string(),
        parameters: HashMap::new(),
        module: local_reference(&inv_params.name),
        connections: conn_map(conns),
    });

    let mut modules = Vec::new();
    modules.push(inv);
    modules.append(&mut bitcells);
    modules.push(precharge);
    modules.push(m);
    modules
}

#[cfg(test)]
mod tests {
    use crate::utils::save_modules;

    use super::*;

    #[test]
    fn test_generate_replica_bitcell_column() -> Result<(), Box<dyn std::error::Error>> {
        let modules = replica_bitcell_column(ReplicaBitcellColumnParams {
            name: "replica_bitcell_column".to_string(),
            num_active_cells: 8,
            height: 16,
        });

        save_modules("replica_bitcell_column", modules)?;
        Ok(())
    }

    #[test]
    fn test_generate_replica_column() -> Result<(), Box<dyn std::error::Error>> {
        let modules = replica_column(ReplicaColumnParams {
            name: "replica_column".to_string(),
            bitcell_params: ReplicaBitcellColumnParams {
                name: "replica_bitcell_column".to_string(),
                num_active_cells: 8,
                height: 16,
            },
        });

        save_modules("replica_column", modules)?;
        Ok(())
    }
}
