use layout21::raw::Layer;
use layout21::raw::LayerPurpose;
use layout21::raw::Layers;
use layout21::raw::LayoutResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub use layout21::raw::Int;

/// The type to use for nonnegative values.
/// Defaults to the same as [`Int`] for now.
pub type Uint = isize;

pub use layout21::raw::Units;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct ContactStack {
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TechConfig {
    pub grid: Int,
    pub tech: String,
    pub gamma: f64,
    pub beta: f64,
    pub units: Units,
    layers: HashMap<String, LayerConfig>,
    spacing: Vec<SpacingConfig>,
    stacks: HashMap<String, ContactStack>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct SpacingConfig {
    pub from: String,
    pub to: String,
    pub dist: Int,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Enclosure {
    pub layer: String,
    pub enclosure: Int,
    pub one_side: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Extension {
    pub layer: String,
    pub extend: Int,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct LayerConfig {
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub width: Int,
    #[serde(default)]
    pub space: Int,
    #[serde(default)]
    pub area: Int,
    #[serde(default)]
    pub enclosures: Vec<Enclosure>,
    #[serde(default)]
    pub extensions: Vec<Extension>,
    pub layernum: i16,
    #[serde(default)]
    pub purposes: Vec<(LayerPurpose, i16)>,
}

impl TechConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let txt = std::fs::read_to_string(path)?;
        Self::from_yaml(&txt)
    }

    pub fn from_toml(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(toml::from_str(s)?)
    }

    pub fn from_yaml(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_yaml::from_str(s)?)
    }

    pub fn to_yaml(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(serde_yaml::to_string(&self)?)
    }

    pub fn layer(&self, l: &str) -> &LayerConfig {
        self.layers.get(l).unwrap()
    }

    pub fn space(&self, from: &str, to: &str) -> Int {
        self.spacing
            .iter()
            .find(|s| (s.from == from && s.to == to) || (s.to == from && s.from == to))
            .take()
            .map(|s| s.dist)
            .unwrap_or_default()
    }

    pub fn scale_pmos(&self, nmos_width: Int) -> Int {
        let pmos_width = (nmos_width as f64 * self.beta) / (self.grid as f64);
        (pmos_width.round() as Int) * self.grid
    }

    pub fn stack(&self, stack: &str) -> &ContactStack {
        self.stacks
            .get(stack)
            .unwrap_or_else(|| panic!("no such stack: {}", stack))
    }

    pub fn get_layers(&self) -> LayoutResult<Layers> {
        let mut layers = Layers::default();
        for (name, cfg) in self.layers.iter() {
            let mut l = Layer::new(cfg.layernum, name);
            for (p, i) in cfg.purposes.iter() {
                l.add_purpose(*i, p.clone())?;
            }
            layers.add(l);
        }
        Ok(layers)
    }
}

impl LayerConfig {
    pub fn extension(&self, l: &str) -> Int {
        self.extensions
            .iter()
            .find(|ext| ext.layer == l)
            .take()
            .map(|ext| ext.extend)
            .unwrap_or_default()
    }

    fn enclosure_inner(&self, l: &str, one_sided: bool) -> Int {
        let x = self
            .enclosures
            .iter()
            .filter(|enc| enc.layer == l)
            .collect::<Vec<_>>();

        if one_sided {
            x.into_iter().map(|x| x.enclosure).max().unwrap_or_default()
        } else {
            x.into_iter()
                .filter(|x| !x.one_side)
                .map(|x| x.enclosure)
                .max()
                .unwrap_or_default()
        }
    }

    pub fn enclosure(&self, l: &str) -> Int {
        self.enclosure_inner(l, false)
    }

    pub fn one_side_enclosure(&self, l: &str) -> Int {
        self.enclosure_inner(l, true)
    }
}

#[cfg(all(test, feature = "sky130"))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sky130_design_rules() -> Result<(), Box<dyn std::error::Error>> {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tech/sky130/drc_config.yaml");
        let tc = TechConfig::load(p)?;
        let yaml = tc.to_yaml()?;

        println!("yaml\n:{}", &yaml);

        let _layers = tc.get_layers()?;

        assert_eq!(&tc.tech, "sky130A");
        assert_eq!(tc.layer("poly").extension("diff"), 130);
        assert_eq!(tc.layer("poly").extension("diff"), 130);

        assert_eq!(tc.layer("poly").extension("diff"), 130);

        assert_eq!(tc.layer("licon").enclosure("poly"), 50);
        assert_eq!(tc.layer("licon").one_side_enclosure("poly"), 80);

        Ok(())
    }

    #[test]
    fn test_serialize_layer() -> Result<(), Box<dyn std::error::Error>> {
        let layer = LayerConfig {
            desc: "test layer".into(),
            width: 200,
            space: 300,
            area: 0,
            layernum: 67,
            purposes: vec![(LayerPurpose::Drawing, 20), (LayerPurpose::Label, 44)],
            enclosures: vec![],
            extensions: vec![],
        };

        let res = toml::to_string(&layer)?;
        println!("{}", res);

        Ok(())
    }
}
