use substrate::{
    pdk::mos::{query::Query, spec::MosKind, MosParams},
    schematic::{circuit::Direction, elements::mos::SchematicMos},
};

use super::Decoder;
use crate::clog2;

impl Decoder {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let out = self.params.tree.root.num;
        let in_bits = clog2(out);

        let vdd = ctx.port("vdd", Direction::InOut);
        let gnd = ctx.port("gnd", Direction::InOut);
        let addr = ctx.bus_port("addr", Direction::Input);
        let addr_b = ctx.bus_port("addr_b", Direction::Input);
        let decode = ctx.bus_port("decode", Direction::Output);
        let decode_b = ctx.bus_port("decode_b", Direction::Output);

        let pmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Pmos).build().unwrap())?
            .id();

        let nmos_id = ctx
            .mos_db()
            .query(Query::builder().kind(MosKind::Nmos).build().unwrap())?
            .id();

        let mut mp = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.pwidth,
            l: length,
            m: 1,
            nf: 1,
            id: pmos_id,
        })?;
        mp.connect_all([("d", &din_b), ("g", &din), ("s", &vdd), ("b", &vdd)]);
        mp.set_name("MP0");
        ctx.add_instance(mp);

        let mut mn = ctx.instantiate::<SchematicMos>(&MosParams {
            w: self.params.nwidth,
            l: length,
            m: 1,
            nf: 1,
            id: nmos_id,
        })?;
        mn.connect_all([("d", &din_b), ("g", &din), ("s", &vss), ("b", &vss)]);
        mn.set_name("MN0");
        ctx.add_instance(mn);

        Ok(())
    }
}

pub struct PhysicalDesignScript;

pub struct PhysicalDesign {
    /// Location of the horizontal power strap
    pub(crate) power_stripe: Span,
    pub(crate) gate_stripe: Span,
    pub(crate) h_metal: LayerKey,
    pub(crate) cut: i64,
    pub(crate) width: i64,
    pub(crate) in_tracks: FixedTracks,
    pub(crate) out_tracks: FixedTracks,
    pub(crate) v_metal: LayerKey,
    pub(crate) v_line: i64,
    pub(crate) v_space: i64,
    pub(crate) m0: LayerKey,
    pub(crate) grid: i64,
    pub(crate) tap_width: i64,
}

impl Script for PhysicalDesignScript {
    type Params = NoParams;
    type Output = PhysicalDesign;

    fn run(
        _params: &Self::Params,
        ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self::Output> {
        let layers = ctx.layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let in_tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 140,
            space: 230,
            span: Span::new(0, 1_200),
            num: 4,
            lower_boundary: Boundary::HalfTrack,
            upper_boundary: Boundary::HalfTrack,
            grid: 5,
        });
        let out_tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 140,
            space: 230,
            span: Span::new(0, 1_200),
            num: 3,
            lower_boundary: Boundary::HalfSpace,
            upper_boundary: Boundary::HalfSpace,
            grid: 5,
        });

        let power_stripe = Span::new(3_400, 4_200);
        let gate_stripe = Span::new(0, 360);

        Ok(PhysicalDesign {
            power_stripe,
            gate_stripe,
            h_metal: m2,
            cut: 1_920,
            width: 1_200,
            v_metal: m1,
            v_line: 140,
            v_space: 140,
            in_tracks,
            out_tracks,
            grid: 5,
            tap_width: 1_300,
            m0,
        })
    }
}
