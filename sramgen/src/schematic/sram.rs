use crate::config::bitcell_array::{BitcellArrayDummyParams, BitcellArrayParams};
use crate::config::col_inv::{ColInvArrayParams, ColInvParams};
use crate::config::decoder::DecoderParams;
use crate::config::dff::DffGridParams;
use crate::config::dout_buffer::{DoutBufArrayParams, DoutBufParams};
use crate::config::gate::{AndParams, GateParams, Size};
use crate::config::inv_chain::InvChainGridParams;
use crate::config::mux::{ReadMuxArrayParams, ReadMuxParams, WriteMuxArrayParams, WriteMuxParams};
use crate::config::precharge::{PrechargeArrayParams, PrechargeParams};
use crate::config::sense_amp::SenseAmpArrayParams;
use crate::config::sram::{ControlMode, SramParams};
use crate::config::wl_driver::{WordlineDriverArrayParams, WordlineDriverParams};
use crate::config::wmask_control::WriteMaskControlParams;
use crate::schematic::bitcell_array::bitcell_array;
use crate::schematic::col_inv::col_inv_array;
use crate::schematic::decoder::{hierarchical_decoder, DecoderTree};
use crate::schematic::dff::dff_grid;
use crate::schematic::dout_buffer::dout_buf_array;
use crate::schematic::inv_chain::inv_chain_grid;
use crate::schematic::mux::read::read_mux_array;
use crate::schematic::mux::write::write_mux_array;
use crate::schematic::precharge::precharge_array;
use crate::schematic::sense_amp::sense_amp_array;
use crate::schematic::vlsir_api::{bus, concat, local_reference, signal, Instance, Module};
use crate::schematic::wl_driver::wordline_driver_array;
use crate::schematic::wmask_control::write_mask_control;
use crate::tech::{
    control_logic_bufbuf_16_ref, openram_dff_ref, sramgen_control_replica_v1_ref,
    sramgen_control_simple_ref,
};

pub fn sram(params: &SramParams) -> Vec<Module> {
    assert!(params.row_bits > 0);
    assert!(params.col_bits > 0);
    assert!(params.col_select_bits <= params.col_bits);
    assert!(params.wmask_width >= 1);

    let row_bits = params.row_bits;
    let col_mask_bits = params.col_select_bits;
    let rows = 1 << params.row_bits;
    let cols = 1 << params.col_bits;
    let mux_ratio = 1 << params.col_select_bits;
    let wmask_width = params.wmask_width;

    let cols_masked = cols / mux_ratio;

    let tree = DecoderTree::new(params.row_bits);
    let decoder_params = DecoderParams {
        name: "hierarchical_decoder".to_string(),
        tree,
        lch: 150,
    };
    let mut decoders = hierarchical_decoder(&decoder_params);

    let mut col_decoders = if mux_ratio > 2 {
        let tree = DecoderTree::new(params.col_select_bits);
        let decoder_params = DecoderParams {
            name: "column_decoder".to_string(),
            tree,
            lch: 150,
        };
        hierarchical_decoder(&decoder_params)
    } else {
        Vec::new()
    };

    let mut wl_drivers = wordline_driver_array(&WordlineDriverArrayParams {
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

    let (replica_cols, dummy_params) = match params.control {
        ControlMode::Simple => (1, BitcellArrayDummyParams::equal(2)),
        ControlMode::ReplicaV1 => (1, BitcellArrayDummyParams::enumerate(2, 2, 1, 2)),
    };

    let bitcells = bitcell_array(&BitcellArrayParams {
        name: "bitcell_array".to_string(),
        rows: rows as usize,
        cols,
        replica_cols,
        dummy_params,
    });

    let pc_cols = if params.control == ControlMode::ReplicaV1 {
        cols + 1
    } else {
        cols
    };

    let mut precharge = precharge_array(&PrechargeArrayParams {
        name: "precharge_array".to_string(),
        width: pc_cols,
        flip_toggle: false,
        instance_params: PrechargeParams {
            name: "precharge".to_string(),
            length: 150,
            pull_up_width: 1_000,
            equalizer_width: 1_000,
        },
    });

    let mut write_muxes = write_mux_array(&WriteMuxArrayParams {
        name: "write_mux_array".to_string(),
        cols,
        mux_ratio,
        wmask_width,
        mux_params: WriteMuxParams {
            name: "write_mux".to_string(),
            length: 150,
            width: 2_000,
            wmask: wmask_width > 1,
        },
    });

    let mut read_muxes = read_mux_array(&ReadMuxArrayParams {
        name: "read_mux_array".to_string(),
        cols,
        mux_ratio,
        mux_params: ReadMuxParams {
            name: "read_mux".to_string(),
            length: 150,
            width: 1_200,
        },
    });

    let mut col_inv = col_inv_array(&ColInvArrayParams {
        name: "col_inv_array".to_string(),
        width: cols_masked as usize,
        mux_ratio,
        instance_params: ColInvParams {
            name: "col_inv".to_string(),
            length: 150,
            nwidth: 1_400,
            pwidth: 2_600,
        },
    });

    let din_dff_params = DffGridParams::builder()
        .name("data_dff_array")
        .rows(2)
        .cols(cols / (2 * mux_ratio))
        .build()
        .unwrap();
    let mut data_dff_array = dff_grid(&din_dff_params);

    let wmask_dff_params = DffGridParams::builder()
        .name("wmask_dff_array")
        .cols(wmask_width)
        .rows(1)
        .build()
        .unwrap();
    let mut wmask_dff_array = dff_grid(&wmask_dff_params);

    let addr_dff_params = DffGridParams::builder()
        .name("addr_dff_array")
        .cols((row_bits + col_mask_bits) as usize)
        .rows(1)
        .build()
        .unwrap();
    let mut addr_dff_array = dff_grid(&addr_dff_params);

    let sense_amp_array = sense_amp_array(&SenseAmpArrayParams {
        name: "sense_amp_array".to_string(),
        width: cols_masked as usize,
        spacing: None,
    });

    let mut dout_buf_array = dout_buf_array(&DoutBufArrayParams {
        name: "dout_buf_array".to_string(),
        width: cols_masked as usize,
        mux_ratio,
        instance_params: DoutBufParams {
            name: "dout_buf".to_string(),
            length: 150,
            nw1: 1_000,
            pw1: 1_600,
            nw2: 2_000,
            pw2: 3_200,
        },
    });

    let mut we_control = write_mask_control(&WriteMaskControlParams {
        name: "we_control".to_string(),
        width: mux_ratio,
        and_params: AndParams {
            name: "we_control_and2".to_string(),
            nand: GateParams {
                name: "we_control_and2_nand".to_string(),
                size: Size {
                    nmos_width: 3_000,
                    pmos_width: 4_000,
                },
                length: 150,
            },
            inv: GateParams {
                name: "we_control_and2_inv".to_string(),
                size: Size {
                    nmos_width: 8_000,
                    pmos_width: 12_000,
                },
                length: 150,
            },
        },
    });

    let inv_chain = inv_chain_grid(&InvChainGridParams {
        name: "control_logic_delay_chain".to_string(),
        rows: 5,
        cols: 9,
    });

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");
    let bank_din = bus("bank_din", cols_masked);
    let bank_din_b = bus("bank_din_b", cols_masked);
    // Not used
    let dff_din_b = bus("dff_din_b", cols_masked);
    let din = bus("din", cols_masked);
    let dout = bus("dout", cols_masked);
    let sa_outp = bus("sa_outp", cols_masked);
    let sa_outn = bus("sa_outn", cols_masked);
    let dout_b = bus("dout_b", cols_masked);
    let we = signal("we");
    let bank_we = signal("bank_we");
    let bank_we_b = signal("bank_we_b");
    let pc_b = signal("pc_b");
    let bl = bus("bl", cols);
    let br = bus("br", cols);
    let bl_read = bus("bl_read", cols_masked);
    let br_read = bus("br_read", cols_masked);
    let wl_en = signal("wl_en");
    let addr = bus("addr", row_bits + col_mask_bits);
    let bank_addr = bus("bank_addr", row_bits + col_mask_bits);
    let bank_addr_b = bus("bank_addr_b", row_bits + col_mask_bits);
    let wl = bus("wl", rows);
    let wl_data = bus("wl_data", rows);
    let wl_data_b = bus("wl_data_b", rows);
    let wr_en = signal("wr_en");
    let write_driver_en = bus("write_driver_en", mux_ratio);
    let sae = signal("sense_amp_en");

    // Only used for replica timing
    let rbl = signal("rbl");
    let rbr = signal("rbr");

    // Only used when mux ratio is greater than 2
    let col_sel = bus("col_sel", mux_ratio);
    let col_sel_b = bus("col_sel_b", mux_ratio);

    // Only used when mux ratio is 2
    let bank_addr_buf = signal("bank_addr_buf");
    let bank_addr_b_buf = signal("bank_addr_b_buf");

    // Only used when wmask groups is greater than 1
    let wmask = bus("wmask", wmask_width);
    let bank_wmask = bus("bank_wmask", wmask_width);
    let bank_wmask_b = bus("bank_wmask_b", wmask_width);

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &vss]);
    m.add_ports_input(&[&clk, &din, &we, &addr]);
    m.add_port_output(&dout);

    if wmask_width > 1 {
        m.add_port_input(&wmask);
    }

    // Data dffs
    let mut inst = Instance::new("din_dffs", local_reference("data_dff_array"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("vss", &vss),
        ("d", &din),
        ("clk", &clk),
        ("q", &bank_din),
        ("q_b", &dff_din_b),
    ]);
    m.add_instance(inst);

    // Address dffs
    let mut inst = Instance::new("addr_dffs", local_reference("addr_dff_array"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("vss", &vss),
        ("d", &addr),
        ("clk", &clk),
        ("q", &bank_addr),
        ("q_b", &bank_addr_b),
    ]);
    m.add_instance(inst);

    // Write mask dffs
    if wmask_width > 1 {
        let mut inst = Instance::new("wmask_dffs", local_reference("wmask_dff_array"));
        inst.add_conns(&[
            ("vdd", &vdd),
            ("vss", &vss),
            ("d", &wmask),
            ("clk", &clk),
            ("q", &bank_wmask),
            ("q_b", &bank_wmask_b),
        ]);
        m.add_instance(inst);
    }

    // we dff
    let mut inst = Instance::new("we_dff", openram_dff_ref());
    inst.add_conns(&[
        ("VDD", &vdd),
        ("GND", &vss),
        ("CLK", &clk),
        ("D", &we),
        ("Q", &bank_we),
        ("Q_N", &bank_we_b),
    ]);
    m.add_instance(inst);

    // Decoder
    let mut inst = Instance::new("decoder", local_reference("hierarchical_decoder"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("gnd", &vss),
        (
            "addr",
            &bank_addr.get_range(col_mask_bits, row_bits + col_mask_bits),
        ),
        (
            "addr_b",
            &bank_addr_b.get_range(col_mask_bits, row_bits + col_mask_bits),
        ),
        ("decode", &wl_data),
        ("decode_b", &wl_data_b),
    ]);
    m.add_instance(inst);

    // Wordline driver array
    let mut inst = Instance::new("wl_driver_array", local_reference("wordline_driver_array"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("vss", &vss),
        ("din", &wl_data),
        ("wl_en", &wl_en),
        ("wl", &wl),
    ]);
    m.add_instance(inst);

    // Bitcells
    let mut inst = Instance::new("bitcells", local_reference("bitcell_array"));
    inst.add_conns(&[
        ("bl", &bl),
        ("br", &br),
        ("rbl", &rbl),
        ("rbr", &rbr),
        ("wl", &wl),
        ("vdd", &vdd),
        ("vss", &vss),
        ("vnb", &vss),
        ("vpb", &vdd),
    ]);
    m.add_instance(inst);

    // Precharge
    let mut inst = Instance::new("precharge_array", local_reference("precharge_array"));
    let (blc, brc) = match params.control {
        ControlMode::Simple => (bl.clone(), br.clone()),
        ControlMode::ReplicaV1 => (
            concat(vec![rbl.clone(), bl.clone()]),
            concat(vec![rbr.clone(), br.clone()]),
        ),
    };
    inst.add_conns(&[("vdd", &vdd), ("en_b", &pc_b), ("bl", &blc), ("br", &brc)]);

    m.add_instance(inst);

    // Column write muxes
    let mut inst = Instance::new("write_mux_array", local_reference("write_mux_array"));
    inst.add_conns(&[
        ("vss", &vss),
        ("bl", &bl),
        ("br", &br),
        ("data", &bank_din),
        ("data_b", &bank_din_b),
        ("we", &write_driver_en),
    ]);
    if wmask_width > 1 {
        inst.add_conns(&[("wmask", &bank_wmask)]);
    }
    m.add_instance(inst);

    // Buffer LSB of address if mux ratio is 2
    if mux_ratio == 2 {
        let mut inst = Instance::new("bank_addr_buf", control_logic_bufbuf_16_ref());
        inst.add_conns(&[
            ("vdd", &vdd),
            ("vss", &vss),
            ("a", &bank_addr.get(0)),
            ("x", &bank_addr_buf),
        ]);
        m.add_instance(inst);

        let mut inst = Instance::new("bank_addr_b_buf", control_logic_bufbuf_16_ref());
        inst.add_conns(&[
            ("vdd", &vdd),
            ("vss", &vss),
            ("a", &bank_addr_b.get(0)),
            ("x", &bank_addr_b_buf),
        ]);
        m.add_instance(inst);
    }

    // Column read muxes
    let sel_b = if mux_ratio == 2 {
        concat(vec![bank_addr_b_buf, bank_addr_buf])
    } else {
        col_sel_b.clone()
    };
    let mut inst = Instance::new("read_mux_array", local_reference("read_mux_array"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("bl", &bl),
        ("br", &br),
        ("bl_out", &bl_read),
        ("br_out", &br_read),
        ("sel_b", &sel_b),
    ]);
    m.add_instance(inst);

    // Column data inverters
    let mut inst = Instance::new("col_inv_array", local_reference("col_inv_array"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("vss", &vss),
        ("din", &bank_din),
        ("din_b", &bank_din_b),
    ]);
    m.add_instance(inst);

    // Sense amplifier array
    let mut inst = Instance::new("sense_amp_array", local_reference("sense_amp_array"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("vss", &vss),
        ("clk", &sae),
        ("bl", &bl_read),
        ("br", &br_read),
        ("data", &sa_outp),
        ("data_b", &sa_outn),
    ]);
    m.add_instance(inst);

    // Data output buffers
    let mut inst = Instance::new("dout_buf_array", local_reference("dout_buf_array"));
    inst.add_conns(&[
        ("vdd", &vdd),
        ("vss", &vss),
        ("din1", &sa_outp),
        ("din2", &sa_outn),
        ("dout1", &dout),
        ("dout2", &dout_b),
    ]);
    m.add_instance(inst);

    // Control logic
    let reference = match params.control {
        ControlMode::Simple => sramgen_control_simple_ref(),
        ControlMode::ReplicaV1 => sramgen_control_replica_v1_ref(),
    };
    let mut inst = Instance::new("control_logic", reference);
    inst.add_conns(&[
        ("clk", &clk),
        ("we", &bank_we),
        ("pc_b", &pc_b),
        ("wl_en", &wl_en),
        ("write_driver_en", &wr_en),
        ("sense_en", &sae),
        ("vdd", &vdd),
        ("vss", &vss),
    ]);

    if params.control == ControlMode::ReplicaV1 {
        inst.add_conns(&[("rbl", &rbl)]);
    }

    m.add_instance(inst);

    // Write enable control
    if mux_ratio == 2 {
        let mut inst = Instance::new("we_control", local_reference("we_control"));
        inst.add_conns(&[
            ("wr_en", &wr_en),
            ("sel", &concat(vec![bank_addr.get(0), bank_addr_b.get(0)])),
            ("write_driver_en", &write_driver_en),
            ("vdd", &vdd),
            ("vss", &vss),
        ]);
        m.add_instance(inst);
    } else {
        let mut inst = Instance::new("column_decoder", local_reference("column_decoder"));
        inst.add_conns(&[
            ("vdd", &vdd),
            ("gnd", &vss),
            ("addr", &bank_addr.get_range(0, col_mask_bits)),
            ("addr_b", &bank_addr_b.get_range(0, col_mask_bits)),
            ("decode", &col_sel),
            ("decode_b", &col_sel_b),
        ]);
        m.add_instance(inst);

        let mut inst = Instance::new("we_control", local_reference("we_control"));
        inst.add_conns(&[
            ("wr_en", &wr_en),
            ("sel", &col_sel),
            ("write_driver_en", &write_driver_en),
            ("vdd", &vdd),
            ("vss", &vss),
        ]);
        m.add_instance(inst);
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
    if wmask_width > 1 {
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
