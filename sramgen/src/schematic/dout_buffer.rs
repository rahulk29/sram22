use pdkprims::mos::MosType;

use crate::config::dout_buffer::{DoutBufArrayParams, DoutBufParams};
use crate::schematic::mos::Mosfet;
use crate::schematic::vlsir_api::{bus, local_reference, signal, Instance, Module};

pub fn dout_buf_array(params: &DoutBufArrayParams) -> Vec<Module> {
    let width = params.width;

    let inv = dout_buf(&params.instance_params);

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din1 = bus("din1", width);
    let din2 = bus("din2", width);
    let dout1 = bus("dout1", width);
    let dout2 = bus("dout2", width);

    let mut m = Module::new(&params.name);

    m.add_ports_input(&[&din1, &din2]);
    m.add_ports_output(&[&dout1, &dout2]);
    m.add_ports_inout(&[&vdd, &vss]);

    for i in 0..width {
        let mut inst = Instance::new(format!("buf_{}", i), local_reference("dout_buf"));
        inst.add_conns(&[
            ("vdd", &vdd),
            ("vss", &vss),
            ("din1", &din1.get(i)),
            ("din2", &din2.get(i)),
            ("dout1", &dout1.get(i)),
            ("dout1", &dout2.get(i)),
        ]);
        m.add_instance(inst);
    }

    vec![inv, m]
}

pub fn dout_buf(params: &DoutBufParams) -> Module {
    let length = params.length;

    let vdd = signal("vdd");
    let vss = signal("vss");
    let din1 = signal("din1");
    let din2 = signal("din2");
    let dout1 = signal("dout1");
    let dout2 = signal("dout2");
    let x1 = signal("x1");
    let x2 = signal("x2");

    let mut m = Module::new(&params.name);

    for (din, x, dout, suffix) in [(&din1, &x1, &dout1, "1"), (&din2, &x2, &dout2, "2")] {
        m.add_instance(
            Mosfet {
                name: format!("MP1{}", suffix),
                width: params.pw1,
                length,
                drain: x.clone(),
                source: vdd.clone(),
                gate: din.clone(),
                body: vdd.clone(),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m.add_instance(
            Mosfet {
                name: format!("MN1{}", suffix),
                width: params.nw1,
                length,
                drain: x.clone(),
                source: vss.clone(),
                gate: din.clone(),
                body: vss.clone(),
                mos_type: MosType::Nmos,
            }
            .into(),
        );

        m.add_instance(
            Mosfet {
                name: format!("MP2{}", suffix),
                width: params.pw2,
                length,
                drain: dout.clone(),
                source: vdd.clone(),
                gate: x.clone(),
                body: vdd.clone(),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m.add_instance(
            Mosfet {
                name: format!("MN2{}", suffix),
                width: params.nw2,
                length,
                drain: dout.clone(),
                source: vss.clone(),
                gate: x.clone(),
                body: vss.clone(),
                mos_type: MosType::Nmos,
            }
            .into(),
        );
    }

    m
}
