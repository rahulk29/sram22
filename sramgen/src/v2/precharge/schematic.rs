use substrate::schematic::circuit::Direction;

use super::Precharge;

impl Precharge {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let length = self.params.length;

        let vdd = ctx.port("vin", Direction::Inout);
        let bl = ctx.port("bl", Direction::Inout);
        let br = ctx.port("br", Direction::Inout);
        let en_b = ctx.port("en_b", Direction::Input);

        let mut m = Module::new(&params.name);
        m.add_ports_inout(&[&vdd, &bl, &br]);
        m.add_port_input(&en_b);

        m.add_instance(
            Mosfet {
                name: "bl_pull_up".to_string(),
                width: params.pull_up_width,
                length,
                drain: bl.clone(),
                source: vdd.clone(),
                gate: en_b.clone(),
                body: vdd.clone(),
                mos_type: MosType::Pmos,
            }
            .into(),
        );
        m.add_instance(
            Mosfet {
                name: "br_pull_up".to_string(),
                width: params.pull_up_width,
                length,
                drain: br.clone(),
                source: vdd.clone(),
                gate: en_b.clone(),
                body: vdd.clone(),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m.add_instance(
            Mosfet {
                name: "equalizer".to_string(),
                width: params.equalizer_width,
                length,
                drain: bl.clone(),
                source: br.clone(),
                gate: en_b.clone(),
                body: vdd.clone(),
                mos_type: MosType::Pmos,
            }
            .into(),
        );

        m
    }
}
