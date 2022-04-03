use layout21::raw::{Cell, Element, LayerPurpose, Layout, LayoutResult, Point, Rect, Shape};
use layout21::utils::Ptr;

use crate::config::Int;
use crate::mos::MosType;
use crate::{
    config::TechConfig,
    mos::{MosError, MosParams, MosResult},
    Pdk,
};

#[cfg(test)]
mod tests;

const SKY130_DRC_CONFIG_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tech/sky130/drc_config.toml"
));

fn tech_config() -> TechConfig {
    TechConfig::from_toml(SKY130_DRC_CONFIG_TOML).expect("failed to load sky130A tech config")
}

pub fn pdk() -> LayoutResult<Pdk> {
    Pdk::new(tech_config())
}

impl Pdk {
    pub fn draw_sky130_mos(&self, params: MosParams) -> MosResult<Ptr<Cell>> {
        params.validate()?;

        let mut elems = Vec::new();

        let tc = self.config.read().unwrap();
        let layers = self.layers.read().unwrap();

        let poly = layers.keyname("poly").unwrap();
        let ndiff = layers.keyname("ndiff").unwrap();
        let pdiff = layers.keyname("pdiff").unwrap();

        let nf = params.fingers();

        // Diff length perpendicular to gates
        let diff_perp = 2 * ndiff_edge_to_gate(&tc)
            + nf * tc.layer("poly").width
            + (nf - 1) * finger_space(&tc);

        let mut prev = None;
        let x0 = 0;
        let mut cx = x0;
        let y0 = 0;
        for d in params.devices.iter() {
            if let Some(mt) = prev {
                if mt != d.mos_type {
                    cx += ndiff_to_pdiff(&tc);
                } else {
                    cx += tc.layer("ndiff").space;
                }
            }

            let layer = match d.mos_type {
                MosType::Nmos => ndiff,
                MosType::Pmos => pdiff,
            };

            let rect = Rect {
                p0: Point::new(cx, y0),
                p1: Point::new(cx + d.width, y0 + diff_perp),
            };

            elems.push(Element {
                net: None,
                layer,
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(rect),
            });

            cx += d.width;

            prev = Some(d.mos_type);
        }

        let xpoly = x0 - tc.layer("poly").extension("ndiff");
        let mut ypoly = y0 + ndiff_edge_to_gate(&tc);
        let wpoly = cx - xpoly + tc.layer("poly").extension("ndiff");
        for _ in 0..nf {
            let rect = Rect {
                p0: Point::new(xpoly, ypoly),
                p1: Point::new(xpoly + wpoly, ypoly + params.length()),
            };

            elems.push(Element {
                net: None,
                layer: poly,
                purpose: LayerPurpose::Drawing,
                inner: Shape::Rect(rect),
            });

            ypoly += params.length();
            ypoly += finger_space(&tc);
        }

        let layout = Layout {
            name: "ptx".to_string(),
            insts: vec![],
            annotations: vec![],
            elems,
        };

        let cell = Cell {
            name: "ptx".to_string(),
            abs: None,
            layout: Some(layout),
        };

        Ok(Ptr::new(cell))
    }
}

pub fn finger_space(tc: &TechConfig) -> Int {
    [
        2 * tc.space("gate", "licon") + tc.layer("li").width,
        tc.layer("poly").space,
    ]
    .into_iter()
    .max()
    .unwrap()
}

pub fn ndiff_edge_to_gate(tc: &TechConfig) -> Int {
    [
        tc.layer("ndiff").extension("poly"),
        tc.space("gate", "licon") + tc.layer("licon").width + tc.layer("licon").enclosure("ndiff"),
    ]
    .into_iter()
    .max()
    .unwrap()
}

pub fn pdiff_edge_to_gate(tc: &TechConfig) -> Int {
    [
        tc.layer("pdiff").extension("poly"),
        tc.space("gate", "licon") + tc.layer("licon").width + tc.layer("licon").enclosure("ndiff"),
    ]
    .into_iter()
    .max()
    .unwrap()
}

pub fn ndiff_to_pdiff(tc: &TechConfig) -> Int {
    tc.space("ndiff", "nwell") + tc.layer("pdiff").enclosure("nwell")
}
