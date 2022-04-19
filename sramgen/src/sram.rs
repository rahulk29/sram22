use std::collections::HashMap;

use vlsir::circuit::{Instance, Module};

use crate::{
    bitcells::{bitcell_array, BitcellArrayParams},
    decoder::{hierarchical_decoder, DecoderParams, DecoderTree},
    mux::{column_mux_4_array, ColumnMuxArrayParams, ColumnMuxParams},
    precharge::{precharge_array, PrechargeArrayParams, PrechargeParams},
    utils::{
        bus, conn_map, conns::conn_slice, local_reference, port_inout, port_input, sig_conn, signal,
    },
};

pub struct SramParams {
    pub row_bits: usize,
    pub col_bits: usize,
    name: String,
}

pub fn sram(params: SramParams) -> Vec<Module> {
    assert!(params.row_bits > 0);
    assert!(params.col_bits > 0);

    // TODO: for now we only support 4:1 sense amps and column muxes
    assert_eq!(params.col_bits, 2);

    let row_bits = params.row_bits as i64;
    let col_bits = params.col_bits as i64;
    let rows = 1 << params.row_bits;
    let cols = 1 << params.col_bits;

    let tree = DecoderTree::new(params.row_bits);
    let decoder_params = DecoderParams {
        tree,
        lch: 150,
        name: "hierarchical_decoder".to_string(),
    };
    let _decoders = hierarchical_decoder(decoder_params);

    let _bitcells = bitcell_array(BitcellArrayParams {
        rows,
        cols,
        name: "bitcell_array".to_string(),
    });

    let _pc = precharge_array(PrechargeArrayParams {
        name: "precharge_array".to_string(),
        width: cols as i64,
        instance_params: PrechargeParams {
            length: 150,
            pull_up_width: 2_000,
            equalizer_width: 1_000,
        },
    });

    let _muxes = column_mux_4_array(ColumnMuxArrayParams {
        name: "column_mux_array".to_string(),
        width: cols as i64,
        instance_params: ColumnMuxParams {
            length: 150,
            width: 2_000,
        },
    });

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din = bus("din", cols as i64);
    let din_b = bus("din_b", cols as i64);
    let dout = bus("dout", (cols / 4) as i64);
    let we = signal("we");
    let cs = signal("cs");
    let _pc_b = signal("pc_b");
    let _pc = signal("pc");
    let _bl_out = bus("bl_out", (cols / 4) as i64);
    let _br_out = bus("br_out", (cols / 4) as i64);
    let addr = bus("addr", row_bits + col_bits);
    let addr_b = bus("addr_b", row_bits + col_bits);
    let _wl = bus("wl", rows as i64);

    let ports = vec![
        port_inout(&vdd),
        port_inout(&din),
        port_inout(&din_b),
        port_inout(&dout),
        port_input(&we),
        port_input(&cs),
        port_input(&addr),
        port_input(&addr_b),
    ];

    let mut m = Module {
        name: params.name,
        ports,
        signals: vec![],
        instances: vec![],
        parameters: vec![],
    };

    // Decoder
    let mut conns = HashMap::new();
    conns.insert("vdd", sig_conn(&vdd));
    conns.insert("gnd", sig_conn(&vss));
    conns.insert(
        "addr",
        conn_slice("addr", row_bits + col_bits - 1, col_bits),
    );
    conns.insert(
        "addr_b",
        conn_slice("addr_b", row_bits + col_bits - 1, col_bits),
    );
    m.instances.push(Instance {
        name: "decoder".to_string(),
        module: local_reference("hierarchical_decoder"),
        connections: conn_map(conns),
        parameters: HashMap::new(),
    });

    vec![m]
}

#[cfg(test)]
mod tests {}
