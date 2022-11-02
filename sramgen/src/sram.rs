use std::collections::HashMap;

use vlsir::circuit::{Concat, Connection, Instance, Module};

use crate::bitcells::{bitcell_array, BitcellArrayParams};
use crate::col_inv::{col_inv_array, ColInvArrayParams, ColInvParams};
use crate::decoder::{hierarchical_decoder, DecoderParams, DecoderTree};
use crate::dff::dff_array;
use crate::gate::{AndParams, Size};
use crate::mux::{
    column_read_mux_2_array, column_write_mux_2_array, ColumnMuxArrayParams, ColumnMuxParams,
};
use crate::precharge::{precharge_array, PrechargeArrayParams, PrechargeParams};
use crate::sense_amp::{sense_amp_array, SenseAmpArrayParams};
use crate::tech::{openram_dff_ref, sramgen_control_ref};
use crate::utils::conns::conn_slice;
use crate::utils::{
    bus, conn_map, local_reference, port_inout, port_input, port_output, sig_conn, signal,
};
use crate::wl_driver::{wordline_driver_array, WordlineDriverArrayParams, WordlineDriverParams};
use crate::wmask_control::{write_mask_control, WriteMaskControlParams};

use crate::dff::DffArrayParams;

pub struct SramParams {
    pub row_bits: usize,
    pub col_bits: usize,
    pub col_mask_bits: usize,
    pub name: String,
}

pub fn sram(params: SramParams) -> Vec<Module> {
    assert!(params.row_bits > 0);
    assert!(params.col_bits > 0);
    assert!(params.col_mask_bits <= params.col_bits);

    // TODO: for now we only support 2:1 sense amps and column muxes
    assert_eq!(params.col_mask_bits, 1);

    let row_bits = params.row_bits as i64;
    let col_mask_bits = params.col_mask_bits as i64;
    let rows = 1 << params.row_bits;
    let cols = 1 << params.col_bits;
    let col_mux_ratio = 1 << params.col_mask_bits;

    let cols_masked = (cols / col_mux_ratio) as i64;

    let tree = DecoderTree::new(params.row_bits);
    let decoder_params = DecoderParams {
        tree,
        lch: 150,
        name: "hierarchical_decoder".to_string(),
    };
    let mut decoders = hierarchical_decoder(decoder_params);

    let mut wl_drivers = wordline_driver_array(WordlineDriverArrayParams {
        name: "wordline_driver_array".to_string(),
        width: rows,
        instance_params: WordlineDriverParams {
            name: "wordline_driver".to_string(),
            length: 150,
            inv_size: Size {
                pmos_width: 2_400,
                nmos_width: 1_600,
            },
            nand_size: Size {
                pmos_width: 2_400,
                nmos_width: 3_200,
            },
        },
    });

    let bitcells = bitcell_array(BitcellArrayParams {
        rows: rows as usize,
        cols,
        name: "bitcell_array".to_string(),
    });

    let mut precharge = precharge_array(PrechargeArrayParams {
        name: "precharge_array".to_string(),
        width: cols,
        instance_params: PrechargeParams {
            name: "precharge".to_string(),
            length: 150,
            pull_up_width: 1_000,
            equalizer_width: 1_000,
        },
    });

    let mut write_muxes = column_write_mux_2_array(ColumnMuxArrayParams {
        name: "column_write_mux_2_array".to_string(),
        width: cols as i64,
        instance_params: ColumnMuxParams {
            length: 150,
            width: 2_000,
        },
    });

    let mut read_muxes = column_read_mux_2_array(ColumnMuxArrayParams {
        name: "column_read_mux_2_array".to_string(),
        width: cols as i64,
        instance_params: ColumnMuxParams {
            length: 150,
            width: 2_000,
        },
    });

    let mut col_inv = col_inv_array(ColInvArrayParams {
        name: "col_inv_array".to_string(),
        width: cols_masked,
        instance_params: ColInvParams {
            length: 150,
            nwidth: 1_400,
            pwidth: 2_600,
        },
    });

    let mut data_dff_array = dff_array(DffArrayParams {
        name: "data_dff_array".to_string(),
        width: cols / 2,
    });

    let mut addr_dff_array = dff_array(DffArrayParams {
        name: "addr_dff_array".to_string(),
        width: (row_bits + col_mask_bits) as usize,
    });

    let sense_amp_array = sense_amp_array(SenseAmpArrayParams {
        name: "sense_amp_array".to_string(),
        width: (cols / col_mux_ratio) as i64,
    });

    let mut write_mask_control = write_mask_control(WriteMaskControlParams {
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

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");
    let bank_din = bus("bank_din", cols_masked as i64);
    let bank_din_b = bus("bank_din_b", cols_masked as i64);
    // Not used
    let dff_din_b = bus("dff_din_b", cols_masked as i64);
    let din = bus("din", cols_masked as i64);
    let dout = bus("dout", cols_masked);
    let dout_b = bus("dout_b", cols_masked);
    let we = signal("we");
    let bank_we = signal("bank_we");
    let bank_we_b = signal("bank_we_b");
    let pc_b = signal("pc_b");
    let bl = bus("bl", cols as i64);
    let br = bus("br", cols as i64);
    let bl_read = bus("bl_read", cols_masked);
    let br_read = bus("br_read", cols_masked);
    let wl_en = signal("wl_en");
    let addr = bus("addr", row_bits + col_mask_bits);
    let bank_addr = bus("bank_addr", row_bits + col_mask_bits);
    let bank_addr_b = bus("bank_addr_b", row_bits + col_mask_bits);
    let wl = bus("wl", rows as i64);
    let wl_data = bus("wl_data", rows as i64);
    let wr_en = signal("wr_en");
    let write_driver_en = bus("write_driver_en", 2);
    let sae = signal("sense_amp_en");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&clk),
        port_input(&din),
        port_output(&dout),
        port_input(&we),
        port_input(&addr),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    // Data dffs
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("d", sig_conn(&din));
    conns.insert("clk", sig_conn(&clk));
    conns.insert("q", sig_conn(&bank_din));
    conns.insert("q_b", sig_conn(&dff_din_b));
    m.instances.push(Instance {
        name: "din_dffs".to_string(),
        module: local_reference("data_dff_array"),
        parameters: HashMap::new(),
        connections: conn_map(conns),
    });

    // Address dffs
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("d", sig_conn(&addr));
    conns.insert("clk", sig_conn(&clk));
    conns.insert("q", sig_conn(&bank_addr));
    conns.insert("q_b", sig_conn(&bank_addr_b));
    m.instances.push(Instance {
        name: "addr_dffs".to_string(),
        module: local_reference("addr_dff_array"),
        parameters: HashMap::new(),
        connections: conn_map(conns),
    });

    // we dff
    let mut connections = HashMap::new();
    connections.insert("VDD".to_string(), sig_conn(&vdd));
    connections.insert("GND".to_string(), sig_conn(&vss));
    connections.insert("CLK".to_string(), sig_conn(&clk));
    connections.insert("D".to_string(), sig_conn(&we));
    connections.insert("Q".to_string(), sig_conn(&bank_we));
    connections.insert("Q_N".to_string(), sig_conn(&bank_we_b));

    m.instances.push(Instance {
        name: "we_dff".to_string(),
        module: Some(openram_dff_ref()),
        parameters: HashMap::new(),
        connections,
    });

    // Decoder
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("gnd", sig_conn(&vss));
    conns.insert(
        "addr",
        conn_slice("bank_addr", row_bits + col_mask_bits - 1, col_mask_bits),
    );
    conns.insert(
        "addr_b",
        conn_slice("bank_addr_b", row_bits + col_mask_bits - 1, col_mask_bits),
    );
    conns.insert("decode", sig_conn(&wl_data));

    m.instances.push(Instance {
        name: "decoder".to_string(),
        module: local_reference("hierarchical_decoder"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Wordline driver array
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("din", sig_conn(&wl_data));
    conns.insert("wl_en", sig_conn(&wl_en));
    conns.insert("wl", sig_conn(&wl));
    m.instances.push(Instance {
        name: "wl_driver_array".to_string(),
        module: local_reference("wordline_driver_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Bitcells
    let mut conns = HashMap::new();
    conns.insert("bl", sig_conn(&bl));
    conns.insert("br", sig_conn(&br));
    conns.insert("wl", sig_conn(&wl));
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("vnb", sig_conn(&vss));
    conns.insert("vpb", sig_conn(&vdd));
    m.instances.push(Instance {
        name: "bitcells".to_string(),
        module: local_reference("bitcell_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Precharge
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("en_b", sig_conn(&pc_b));
    conns.insert("bl", sig_conn(&bl));
    conns.insert("br", sig_conn(&br));
    m.instances.push(Instance {
        name: "precharge_array".to_string(),
        module: local_reference("precharge_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Column write muxes
    let mut conns = HashMap::new();
    conns.insert("vss", sig_conn(&vss));
    conns.insert("bl", sig_conn(&bl));
    conns.insert("br", sig_conn(&br));
    conns.insert("data", sig_conn(&bank_din));
    conns.insert("data_b", sig_conn(&bank_din_b));
    conns.insert("we_0_0", conn_slice("write_driver_en", 0, 0));
    conns.insert("we_1_0", conn_slice("write_driver_en", 1, 1));
    m.instances.push(Instance {
        name: "column_write_mux_2_array".to_string(),
        module: local_reference("column_write_mux_2_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Column read muxes
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("bl", sig_conn(&bl));
    conns.insert("br", sig_conn(&br));
    conns.insert("bl_out", sig_conn(&bl_read));
    conns.insert("br_out", sig_conn(&br_read));
    conns.insert(
        "sel",
        Connection {
            stype: Some(vlsir::circuit::connection::Stype::Concat(Concat {
                parts: vec![
                    conn_slice("bank_addr", 0, 0),
                    conn_slice("bank_addr_b", 0, 0),
                ],
            })),
        },
    );
    m.instances.push(Instance {
        name: "column_read_mux_2_array".to_string(),
        module: local_reference("column_read_mux_2_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Column data inverters
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("din", sig_conn(&bank_din));
    conns.insert("din_b", sig_conn(&bank_din_b));
    m.instances.push(Instance {
        name: "col_inv_array".to_string(),
        module: local_reference("col_inv_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Sense amplifier array
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("clk", sig_conn(&sae));
    conns.insert("bl", sig_conn(&bl_read));
    conns.insert("br", sig_conn(&br_read));
    conns.insert("data", sig_conn(&dout));
    conns.insert("data_b", sig_conn(&dout_b));
    m.instances.push(Instance {
        name: "sense_amp_array".to_string(),
        module: local_reference("sense_amp_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Simple control logic
    let conns: HashMap<_, _> = [
        ("clk", sig_conn(&clk)),
        ("we", sig_conn(&bank_we)),
        ("pc_b", sig_conn(&pc_b)),
        ("wl_en", sig_conn(&wl_en)),
        ("write_driver_en", sig_conn(&wr_en)),
        ("sense_en", sig_conn(&sae)),
        ("vdd", sig_conn(&vdd)),
        ("vss", sig_conn(&vss)),
    ]
    .into();
    m.instances.push(Instance {
        name: "sramgen_control_logic".to_string(),
        module: Some(sramgen_control_ref()),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Write mask control
    // (There's no write mask yet; this only handles column selection.)

    let conns = [
        ("wr_en", sig_conn(&wr_en)),
        (
            "sel",
            Connection {
                stype: Some(vlsir::circuit::connection::Stype::Concat(Concat {
                    parts: vec![
                        conn_slice("bank_addr_b", 0, 0),
                        conn_slice("bank_addr", 0, 0),
                    ],
                })),
            },
        ),
        ("write_driver_en", sig_conn(&write_driver_en)),
        ("vdd", sig_conn(&vdd)),
        ("vss", sig_conn(&vss)),
    ];
    m.instances.push(Instance {
        name: "write_mask_control".to_string(),
        module: local_reference("write_mask_control"),
        connections: conn_map(conns.into()),
        parameters: HashMap::new(),
    });

    let mut modules = Vec::new();
    modules.append(&mut decoders);
    modules.append(&mut wl_drivers);
    modules.push(bitcells);
    modules.append(&mut precharge);
    modules.append(&mut read_muxes);
    modules.append(&mut write_muxes);
    modules.append(&mut data_dff_array);
    modules.append(&mut addr_dff_array);
    modules.append(&mut col_inv);
    modules.push(sense_amp_array);
    modules.append(&mut write_mask_control);
    modules.push(m);

    modules
}

#[cfg(test)]
mod tests {
    use crate::utils::save_modules;

    use super::*;

    #[test]
    fn test_netlist_sram_16x16() -> Result<(), Box<dyn std::error::Error>> {
        let modules = sram(SramParams {
            name: "sramgen_sram_16x16".to_string(),
            row_bits: 4,
            col_bits: 4,
            col_mask_bits: 1,
        });

        save_modules("sram_16x16", modules)?;
        Ok(())
    }

    #[test]
    fn test_netlist_sram_4x4() -> Result<(), Box<dyn std::error::Error>> {
        let modules = sram(SramParams {
            name: "sramgen_sram_4x4".to_string(),
            row_bits: 2,
            col_bits: 2,
            col_mask_bits: 1,
        });

        save_modules("sram_4x4", modules)?;
        Ok(())
    }

    #[test]
    fn test_netlist_sram_32x32() -> Result<(), Box<dyn std::error::Error>> {
        let modules = sram(SramParams {
            name: "sramgen_sram_32x32".to_string(),
            row_bits: 5,
            col_bits: 5,
            col_mask_bits: 1,
        });

        save_modules("sram_32x32", modules)?;
        Ok(())
    }

    #[test]
    fn test_netlist_sram_32x64() -> Result<(), Box<dyn std::error::Error>> {
        let modules = sram(SramParams {
            name: "sramgen_sram_32x64".to_string(),
            row_bits: 5,
            col_bits: 6,
            col_mask_bits: 1,
        });

        save_modules("sram_32x64", modules)?;
        Ok(())
    }
}
