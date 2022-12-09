use crate::config::bitcell_array::{BitcellArrayDummyParams, BitcellArrayParams};
use crate::schematic::vlsir_api::{bus, signal, Instance, Module};
use crate::tech::{sram_sp_cell_ref, sram_sp_cell_replica_ref, sram_sp_colend_ref};

pub fn bitcell_array(params: &BitcellArrayParams) -> Module {
    let rows = params.rows;
    let cols = params.cols;
    let replica_cols = params.replica_cols;
    let dummy_params = &params.dummy_params;

    let (dummy_rows_top, dummy_rows_bottom, dummy_cols_left, dummy_cols_right) = {
        let &BitcellArrayDummyParams {
            top,
            bottom,
            left,
            right,
        } = dummy_params;
        (top, bottom, left, right)
    };

    let total_rows = rows + dummy_rows_top + dummy_rows_bottom;
    let total_cols = cols + dummy_cols_left + dummy_cols_right + replica_cols;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let bl = bus("bl", cols);
    let br = bus("br", cols);
    let wl = bus("wl", rows);
    let vnb = signal("vnb");
    let vpb = signal("vpb");
    let rbl = signal("rbl");
    let rbr = signal("rbr");

    let mut m = Module::new(&params.name);

    m.add_ports_inout(&[&vdd, &vss, &bl, &br, &vnb, &vpb]);

    m.add_port_input(&wl);

    if replica_cols > 0 {
        m.add_ports_inout(&[&rbl, &rbr]);
    }

    for i in 0..total_rows {
        for j in 0..total_cols {
            let module = if j < dummy_cols_left + replica_cols
                && i >= dummy_rows_bottom
                && i < rows + dummy_rows_bottom
            {
                sram_sp_cell_replica_ref()
            } else {
                sram_sp_cell_ref()
            };
            let mut inst = Instance::new(format!("bitcell_{}_{}", i, j), module);

            let wl_sig = if i < dummy_rows_bottom || i >= rows + dummy_rows_bottom {
                vss.clone()
            } else {
                wl.get(i - dummy_rows_bottom)
            };

            let (bl_sig, br_sig) =
                if j < dummy_cols_left || j >= cols + dummy_cols_left + replica_cols {
                    (vdd.clone(), vdd.clone())
                } else if j < dummy_cols_left + replica_cols {
                    (rbl.clone(), rbr.clone())
                } else {
                    (
                        bl.get(j - dummy_cols_left - replica_cols),
                        br.get(j - dummy_cols_left - replica_cols),
                    )
                };

            inst.add_conns(&[
                ("VDD", &vdd),
                ("VSS", &vss),
                ("VNB", &vnb),
                ("VPB", &vpb),
                ("WL", &wl_sig),
                ("BL", &bl_sig),
                ("BR", &br_sig),
            ]);

            m.add_instance(inst);
        }
    }

    for i in 0..total_cols {
        // .subckt sky130_fd_bd_sram__sram_sp_colend BL1 VPWR VGND BL0
        let is_dummy = i < dummy_cols_left || i >= cols + dummy_cols_left + replica_cols;
        let is_replica = !is_dummy && i < dummy_cols_left + replica_cols;

        let (bl0_sig, bl1_sig) = if is_dummy {
            (vdd.clone(), vdd.clone())
        } else if is_replica {
            (rbl.clone(), rbr.clone())
        } else {
            (
                bl.get(i - dummy_cols_left - replica_cols),
                br.get(i - dummy_cols_left - replica_cols),
            )
        };

        let conns = vec![
            ("BL1", &bl1_sig),
            ("BL0", &bl0_sig),
            ("VPWR", &vdd),
            ("VGND", &vss),
            ("VNB", &vnb),
            ("VPB", &vpb),
        ];

        let mut inst = Instance::new(format!("colend_{}_bot", i), sram_sp_colend_ref());
        inst.add_conns(&conns);

        m.add_instance(inst);

        let mut inst = Instance::new(format!("colend_{}_top", i), sram_sp_colend_ref());
        inst.add_conns(&conns);

        m.add_instance(inst);
    }

    m
}
