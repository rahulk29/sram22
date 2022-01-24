#[cfg(test)]
use micro_hdl::backend::spice::SpiceBackend;

use super::*;

#[test]
fn test_netlist_sram_bitcell_array() {
    let out = <Vec<u8>>::new();
    let mut b = SpiceBackend::new(out);

    let rows = 32;
    let cols = 64;

    let wls = b.top_level_bus(rows);
    let bls = b.top_level_bus(cols);
    let blbs = b.top_level_bus(cols);
    let vdd = b.top_level_signal();
    let gnd = b.top_level_signal();

    let array = BitcellArray::instance()
        .dims(ArrayDimensions { rows, cols })
        .wordlines(wls)
        .bitlines(bls)
        .bitline_bs(blbs)
        .vdd(vdd)
        .gnd(gnd)
        .build();

    b.netlist(array);
    let out = b.output();

    let out = String::from_utf8(out).unwrap();
    println!("{}", out);
}
