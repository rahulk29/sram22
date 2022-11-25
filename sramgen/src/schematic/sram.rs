use std::collections::HashMap;

use vlsir::circuit::{Concat, Connection, Instance, Module};

use crate::config::sram::SramParams;
use crate::layout::inv_chain::InvChainGridParams;
use crate::schematic::bitcell_array::{bitcell_array, BitcellArrayParams};
use crate::schematic::col_inv::{col_inv_array, ColInvArrayParams, ColInvParams};
use crate::schematic::conns::{
    bus, conn_map, conn_slice, port_inout, port_input, port_output, sig_conn, signal,
};
use crate::schematic::decoder::{hierarchical_decoder, DecoderParams, DecoderTree};
use crate::schematic::dff::dff_array;
use crate::schematic::dout_buffer::{dout_buf_array, DoutBufArrayParams, DoutBufParams};
use crate::schematic::gate::{AndParams, GateParams, Size};
use crate::schematic::inv_chain::inv_chain_grid;
use crate::schematic::mux::read::read_mux_array;
use crate::schematic::mux::write::{write_mux_array, ArrayParams, WriteMuxParams};
use crate::schematic::precharge::{precharge_array, PrechargeArrayParams, PrechargeParams};
use crate::schematic::sense_amp::{sense_amp_array, SenseAmpArrayParams};
use crate::schematic::wl_driver::{
    wordline_driver_array, WordlineDriverArrayParams, WordlineDriverParams,
};
use crate::schematic::wmask_control::{write_mask_control, WriteMaskControlParams};
use crate::schematic::{local_reference, mux};
use crate::tech::{openram_dff_ref, sramgen_control_ref};

use crate::schematic::dff::DffArrayParams;

pub fn sram(params: &SramParams) -> Vec<Module> {
    assert!(params.row_bits > 0);
    assert!(params.col_bits > 0);
    assert!(params.col_select_bits <= params.col_bits);
    assert!(params.wmask_width >= 1);

    let row_bits = params.row_bits as i64;
    let col_mask_bits = params.col_select_bits as i64;
    let rows = 1 << params.row_bits;
    let cols = 1 << params.col_bits;
    let mux_ratio = 1 << params.col_select_bits;
    let wmask_groups = params.wmask_width;

    let cols_masked = (cols / mux_ratio) as i64;

    let tree = DecoderTree::new(params.row_bits);
    let decoder_params = DecoderParams {
        tree,
        lch: 150,
        name: "hierarchical_decoder".to_string(),
    };
    let mut decoders = hierarchical_decoder(decoder_params);

    let mut col_decoders = if mux_ratio > 2 {
        let tree = DecoderTree::new(params.col_select_bits);
        let decoder_params = DecoderParams {
            tree,
            lch: 150,
            name: "column_decoder".to_string(),
        };
        hierarchical_decoder(decoder_params)
    } else {
        Vec::new()
    };

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
        dummy_rows: 2,
        dummy_cols: 2,
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

    let mut write_muxes = write_mux_array(ArrayParams {
        cols,
        mux_ratio,
        wmask_groups,
        mux_params: WriteMuxParams {
            length: 150,
            width: 2_000,
            wmask: wmask_groups > 1,
        },
    });

    let mut read_muxes = read_mux_array(mux::read::ArrayParams {
        cols,
        mux_ratio,
        mux_params: mux::read::Params {
            length: 150,
            width: 1_200,
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
        width: cols / mux_ratio,
    });

    let mut wmask_dff_array = dff_array(DffArrayParams {
        name: "wmask_dff_array".to_string(),
        width: wmask_groups,
    });

    let mut addr_dff_array = dff_array(DffArrayParams {
        name: "addr_dff_array".to_string(),
        width: (row_bits + col_mask_bits) as usize,
    });

    let sense_amp_array = sense_amp_array(SenseAmpArrayParams {
        name: "sense_amp_array".to_string(),
        width: cols_masked,
    });

    let mut dout_buf_array = dout_buf_array(DoutBufArrayParams {
        name: "dout_buf_array".to_string(),
        width: cols_masked,
        instance_params: DoutBufParams {
            length: 150,
            nw1: 1_000,
            pw1: 1_600,
            nw2: 2_000,
            pw2: 3_200,
        },
    });

    let mut we_control = write_mask_control(WriteMaskControlParams {
        name: "we_control".to_string(),
        width: mux_ratio as i64,
        and_params: AndParams {
            name: "we_control_and2".to_string(),
            nand: GateParams {
                name: "we_control_and2_nand".to_string(),
                size: Size {
                    nmos_width: 1_200,
                    pmos_width: 1_800,
                },
                length: 150,
            },
            inv: GateParams {
                name: "we_control_and2_inv".to_string(),
                size: Size {
                    nmos_width: 1_200,
                    pmos_width: 1_800,
                },
                length: 150,
            },
        },
    });

    let inv_chain = inv_chain_grid(InvChainGridParams {
        prefix: "control_logic_delay_chain",
        rows: 5,
        cols: 9,
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
    let sa_outp = bus("sa_outp", cols_masked);
    let sa_outn = bus("sa_outn", cols_masked);
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
    let wl_data_b = bus("wl_data_b", rows as i64);
    let wr_en = signal("wr_en");
    let write_driver_en = bus("write_driver_en", mux_ratio as i64);
    let sae = signal("sense_amp_en");

    // Only used when mux ratio is greater than 2
    let col_sel = bus("col_sel", mux_ratio as i64);
    let col_sel_b = bus("col_sel_b", mux_ratio as i64);

    // Only used when wmask groups is greater than 1
    let wmask = bus("wmask", wmask_groups as i64);
    let bank_wmask = bus("bank_wmask", wmask_groups as i64);
    let bank_wmask_b = bus("bank_wmask_b", wmask_groups as i64);

    let mut ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&clk),
        port_input(&din),
        port_output(&dout),
        port_input(&we),
        port_input(&addr),
    ];

    if wmask_groups > 1 {
        ports.push(port_input(&wmask));
    }

    let mut m = Module {
        name: params.name.clone(),
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

    // Write mask dffs
    if wmask_groups > 1 {
        let mut conns = HashMap::new();
        conns.insert("vdd", sig_conn(&vdd));
        conns.insert("vss", sig_conn(&vss));
        conns.insert("d", sig_conn(&wmask));
        conns.insert("clk", sig_conn(&clk));
        conns.insert("q", sig_conn(&bank_wmask));
        conns.insert("q_b", sig_conn(&bank_wmask_b));
        m.instances.push(Instance {
            name: "wmask_dffs".to_string(),
            module: local_reference("wmask_dff_array"),
            parameters: HashMap::new(),
            connections: conn_map(conns),
        });
    }

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
    conns.insert("decode_b", sig_conn(&wl_data_b));

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
    conns.insert("we", sig_conn(&write_driver_en));
    if wmask_groups > 1 {
        conns.insert("wmask", sig_conn(&bank_wmask));
    }
    m.instances.push(Instance {
        name: "write_mux_array".to_string(),
        module: local_reference("write_mux_array"),
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
        "sel_b",
        if mux_ratio == 2 {
            Connection {
                stype: Some(vlsir::circuit::connection::Stype::Concat(Concat {
                    parts: vec![
                        conn_slice("bank_addr_b", 0, 0),
                        conn_slice("bank_addr", 0, 0),
                    ],
                })),
            }
        } else {
            sig_conn(&col_sel_b)
        },
    );
    m.instances.push(Instance {
        name: "read_mux_array".to_string(),
        module: local_reference("read_mux_array"),
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
    conns.insert("data", sig_conn(&sa_outp));
    conns.insert("data_b", sig_conn(&sa_outn));
    m.instances.push(Instance {
        name: "sense_amp_array".to_string(),
        module: local_reference("sense_amp_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Data output buffers
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("din1", sig_conn(&sa_outp));
    conns.insert("din2", sig_conn(&sa_outn));
    conns.insert("dout1", sig_conn(&dout));
    conns.insert("dout2", sig_conn(&dout_b));
    m.instances.push(Instance {
        name: "dout_buf_array".to_string(),
        module: local_reference("dout_buf_array"),
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

    // Write enable control
    if mux_ratio == 2 {
        let conns = [
            ("wr_en", sig_conn(&wr_en)),
            (
                "sel",
                Connection {
                    stype: Some(vlsir::circuit::connection::Stype::Concat(Concat {
                        parts: vec![
                            conn_slice("bank_addr", 0, 0),
                            conn_slice("bank_addr_b", 0, 0),
                        ],
                    })),
                },
            ),
            ("write_driver_en", sig_conn(&write_driver_en)),
            ("vdd", sig_conn(&vdd)),
            ("vss", sig_conn(&vss)),
        ];
        m.instances.push(Instance {
            name: "we_control".to_string(),
            module: local_reference("we_control"),
            connections: conn_map(conns.into()),
            parameters: HashMap::new(),
        });
    } else {
        let mut conns = HashMap::new();
        conns.insert("vdd", sig_conn(&vdd));
        conns.insert("gnd", sig_conn(&vss));
        conns.insert("addr", conn_slice("bank_addr", col_mask_bits - 1, 0));
        conns.insert("addr_b", conn_slice("bank_addr_b", col_mask_bits - 1, 0));
        conns.insert("decode", sig_conn(&col_sel));
        conns.insert("decode_b", sig_conn(&col_sel_b));

        m.instances.push(Instance {
            name: "column_decoder".to_string(),
            module: local_reference("column_decoder"),
            connections: conn_map(conns),
            parameters: HashMap::new(),
        });
        let conns = [
            ("wr_en", sig_conn(&wr_en)),
            ("sel", sig_conn(&col_sel)),
            ("write_driver_en", sig_conn(&write_driver_en)),
            ("vdd", sig_conn(&vdd)),
            ("vss", sig_conn(&vss)),
        ];
        m.instances.push(Instance {
            name: "we_control".to_string(),
            module: local_reference("we_control"),
            connections: conn_map(conns.into()),
            parameters: HashMap::new(),
        });
    }

    let mut modules = Vec::new();
    modules.append(&mut decoders);
    modules.append(&mut col_decoders);
    modules.append(&mut wl_drivers);
    modules.push(bitcells);
    modules.append(&mut precharge);
    modules.append(&mut read_muxes);
    modules.append(&mut write_muxes);
    modules.append(&mut data_dff_array);
    modules.append(&mut addr_dff_array);
    if wmask_groups > 1 {
        modules.append(&mut wmask_dff_array);
    }
    modules.append(&mut col_inv);
    modules.push(sense_amp_array);
    modules.append(&mut dout_buf_array);
    modules.append(&mut we_control);
    modules.push(inv_chain);
    modules.push(m);

    modules
}
