use crate::config::sense_amp::SenseAmpArrayParams;
use crate::schematic::vlsir_api::{bus, signal, Instance, Module};
use crate::tech::sramgen_sp_sense_amp_ref;

pub fn sense_amp_array(params: &SenseAmpArrayParams) -> Module {
    let width = params.width;

    assert!(width > 0);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let clk = signal("clk");
    let bl = bus("bl", width);
    let br = bus("br", width);
    let data = bus("data", width);
    let data_b = bus("data_b", width);

    let mut m = Module::new(&params.name);
    m.add_ports_inout(&[&vdd, &vss]);
    m.add_ports_input(&[&clk, &bl, &br]);
    m.add_ports_output(&[&data, &data_b]);

    for i in 0..width {
        let mut inst = Instance::new(format!("sense_amp_{i}"), sramgen_sp_sense_amp_ref());
        inst.add_conns(&[
            ("clk", &clk),
            ("inn", &br.get(i)),
            ("inp", &bl.get(i)),
            ("outp", &data.get(i)),
            ("outn", &data_b.get(i)),
            ("VDD", &vdd),
            ("VSS", &vss),
        ]);

        m.add_instance(inst);
    }

    m
}
