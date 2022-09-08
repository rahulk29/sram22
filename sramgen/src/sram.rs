use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::{
    bitcells::{bitcell_array, BitcellArrayParams},
    decoder::{hierarchical_decoder, DecoderParams, DecoderTree},
    dff::dff_array,
    gate::Size,
    mux::{
        column_read_mux_2_array, column_write_mux_2_array, ColumnMuxArrayParams, ColumnMuxParams,
    },
    precharge::{precharge_array, PrechargeArrayParams, PrechargeParams},
    sense_amp::{sense_amp_array, SenseAmpArrayParams},
    utils::{
        bus, conn_map, conns::conn_slice, local_reference, port_inout, port_input, port_output,
        sig_conn, signal,
    },
    wl_driver::{wordline_driver_array, WordlineDriverArrayParams, WordlineDriverParams},
    write_driver::{bitline_driver_array, BitlineDriverArrayParams, BitlineDriverParams},
};

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
                pmos_width: 2_000,
                nmos_width: 1_000,
            },
            nand_size: Size {
                pmos_width: 2_000,
                nmos_width: 1_000,
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
        width: cols as i64,
        instance_params: PrechargeParams {
            name: "precharge".to_string(),
            length: 150,
            pull_up_width: 2_000,
            equalizer_width: 1_000,
        },
    });

    let mut wr_drivers = bitline_driver_array(BitlineDriverArrayParams {
        name: "write_driver_array".to_string(),
        width: cols_masked as i64,
        instance_params: BitlineDriverParams {
            length: 150,
            width: 1_800,
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

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");
    let clk_b = signal("clk_b");
    let din = bus("din", cols_masked as i64);
    let din_b = bus("din_b", cols_masked as i64);
    let din_in = bus("din_in", cols_masked as i64);
    let dout = bus("dout", cols_masked);
    let dout_b = bus("dout_b", cols_masked);
    let dout_out = bus("dout_out", cols_masked);
    let we = signal("we");
    let cs = signal("cs");
    let pc_b = signal("pc_b");
    let bl = bus("bl", cols as i64);
    let br = bus("br", cols as i64);
    let bl_read = bus("bl_read", cols_masked);
    let br_read = bus("br_read", cols_masked);
    let bl_write = bus("bl_write", cols_masked);
    let br_write = bus("br_write", cols_masked);
    let wl_en = signal("wl_en");
    let addr_in = bus("addr_in", row_bits + col_mask_bits);
    let addr = bus("addr", row_bits + col_mask_bits);
    let addr_b = bus("addr_b", row_bits + col_mask_bits);
    let wl = bus("wl", rows as i64);
    let wl_data = bus("wl_data", rows as i64);
    let wr_drv_en = signal("wr_drv_en");
    let sae = signal("sense_amp_en");

    let ports = vec![
        port_inout(&vdd),
        port_inout(&vss),
        port_input(&clk),
        port_input(&clk_b),
        port_input(&din_in),
        port_output(&dout_out),
        port_input(&we),
        port_input(&cs),
        port_input(&addr_in),
        // control signals
        port_input(&wl_en),
        port_input(&sae),
        port_input(&pc_b),
        port_input(&wr_drv_en),
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
    conns.insert("d", sig_conn(&din_in));
    conns.insert("clk", sig_conn(&clk));
    conns.insert("q", sig_conn(&din));
    conns.insert("q_b", sig_conn(&din_b));
    m.instances.push(Instance {
        name: "din_dffs".to_string(),
        module: local_reference("data_dff_array"),
        parameters: HashMap::new(),
        connections: conn_map(conns),
    });

    let dout_negedge = bus("dout_negedge", cols_masked);
    let dout_negedge_b = bus("dout_negedge_b", cols_masked);

    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("d", sig_conn(&dout));
    conns.insert("clk", sig_conn(&clk_b));
    conns.insert("q", sig_conn(&dout_negedge));
    conns.insert("q_b", sig_conn(&dout_negedge_b));
    m.instances.push(Instance {
        name: "dout_negedge_dffs".to_string(),
        module: local_reference("data_dff_array"),
        parameters: HashMap::new(),
        connections: conn_map(conns),
    });

    let dout_out_b = bus("dout_out_b", (cols / col_mux_ratio) as i64);
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("d", sig_conn(&dout_negedge));
    conns.insert("clk", sig_conn(&clk));
    conns.insert("q", sig_conn(&dout_out));
    conns.insert("q_b", sig_conn(&dout_out_b));
    m.instances.push(Instance {
        name: "dout_dffs".to_string(),
        module: local_reference("data_dff_array"),
        parameters: HashMap::new(),
        connections: conn_map(conns),
    });

    // Address dffs
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
    conns.insert("d", sig_conn(&addr_in));
    conns.insert("clk", sig_conn(&clk));
    conns.insert("q", sig_conn(&addr));
    conns.insert("q_b", sig_conn(&addr_b));
    m.instances.push(Instance {
        name: "addr_dffs".to_string(),
        module: local_reference("addr_dff_array"),
        parameters: HashMap::new(),
        connections: conn_map(conns),
    });

    // Decoder
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("gnd", sig_conn(&vss));
    conns.insert(
        "addr",
        conn_slice("addr", row_bits + col_mask_bits - 1, col_mask_bits),
    );
    conns.insert(
        "addr_b",
        conn_slice("addr_b", row_bits + col_mask_bits - 1, col_mask_bits),
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
    conns.insert("bls", sig_conn(&bl));
    conns.insert("brs", sig_conn(&br));
    conns.insert("wls", sig_conn(&wl));
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("vss", sig_conn(&vss));
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

    // Write driver array
    let mut conns = HashMap::new();
    conns.insert("vss", sig_conn(&vss));
    conns.insert("bl", sig_conn(&bl_write));
    conns.insert("br", sig_conn(&br_write));
    conns.insert("din", sig_conn(&din));
    conns.insert("din_b", sig_conn(&din_b));
    conns.insert("we", sig_conn(&wr_drv_en));
    m.instances.push(Instance {
        name: "write_driver_array".to_string(),
        module: local_reference("write_driver_array"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    // Column write muxes
    let mut conns = HashMap::new();
    conns.insert("vss", sig_conn(&vss));
    conns.insert("bl", sig_conn(&bl));
    conns.insert("br", sig_conn(&br));
    conns.insert("bl_out", sig_conn(&bl_write));
    conns.insert("br_out", sig_conn(&br_write));
    // Note addr is flipped here because devices are NMOS
    conns.insert("sel", conn_slice("addr_b", 0, 0));
    conns.insert("sel_b", conn_slice("addr", 0, 0));
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
    conns.insert("sel", conn_slice("addr", 0, 0));
    conns.insert("sel_b", conn_slice("addr_b", 0, 0));
    m.instances.push(Instance {
        name: "column_read_mux_2_array".to_string(),
        module: local_reference("column_read_mux_2_array"),
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

    let mut modules = Vec::new();
    modules.append(&mut decoders);
    modules.append(&mut wl_drivers);
    modules.append(&mut wr_drivers);
    modules.push(bitcells);
    modules.append(&mut precharge);
    modules.append(&mut read_muxes);
    modules.append(&mut write_muxes);
    modules.append(&mut data_dff_array);
    modules.append(&mut addr_dff_array);
    modules.push(sense_amp_array);
    modules.push(m);

    modules
}

#[cfg(test)]
mod tests {
    use crate::utils::save_modules;

    use super::*;

    #[test]
    fn test_generate_sram_16x16() -> Result<(), Box<dyn std::error::Error>> {
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
    fn test_generate_sram_4x4() -> Result<(), Box<dyn std::error::Error>> {
        let modules = sram(SramParams {
            name: "sramgen_sram_4x4".to_string(),
            row_bits: 2,
            col_bits: 2,
            col_mask_bits: 1,
        });

        save_modules("sram_4x4", modules)?;
        Ok(())
    }
}
