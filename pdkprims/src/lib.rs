use std::{collections::HashMap, path::Path};

use config::TechConfig;
use contact::{Contact, ContactParams};
use layout21::{
    raw::{Cell, LayerKey, Layers, LayoutResult, Library},
    utils::Ptr,
};

pub type Ref<T> = std::sync::Arc<T>;

pub mod config;
pub mod contact;
pub mod geometry;
pub mod mos;
pub mod tech;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdk {
    pub config: Ptr<TechConfig>,
    pub layers: Ptr<Layers>,
    contacts: Ptr<HashMap<ContactParams, Ref<Contact>>>,
}

impl Pdk {
    pub fn new(config: TechConfig) -> LayoutResult<Self> {
        let layers = Ptr::new(config.get_layers()?);
        let config = Ptr::new(config);
        Ok(Self {
            config,
            layers,
            contacts: Ptr::new(HashMap::new()),
        })
    }
    #[inline]
    pub fn config(&self) -> Ptr<TechConfig> {
        Ptr::clone(&self.config)
    }

    #[inline]
    pub fn layers(&self) -> Ptr<Layers> {
        Ptr::clone(&self.layers)
    }

    pub fn cell_to_gds(
        &self,
        cell: Ptr<Cell>,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cell_name = {
            let cell = cell.read().unwrap();
            cell.name.to_owned()
        };
        let mut lib = Library::new(&cell_name, self.config.read().unwrap().units);
        lib.layers = self.layers();
        lib.cells.push(cell);
        let gds = lib.to_gds()?;
        gds.save(path)?;
        Ok(())
    }

    pub fn get_layerkey(&self, layer: &str) -> Option<LayerKey> {
        let layers = self.layers.read().unwrap();
        layers.keyname(layer)
    }
}

// #[cfg(test)]
// mod tests {
//     use layout21::{raw::{Library, Cell}, utils::Ptr};
//
//     #[test]
//     fn test_draw_mos() -> Result<(), Box<dyn std::error::Error>> {
//         let tc = crate::sky130_config();
//         let layout = crate::draw_mos((), &tc)?;
//         let mut cell = Cell::new("ptx");
//         cell.layout = Some(layout);
//
//         let mut lib = Library::new("test_draw_mos", tc.units);
//         lib.layers = Ptr::new(tc.get_layers()?);
//         lib.cells.push(Ptr::new(cell));
//         let gds = lib.to_gds()?;
//         gds.save("hi.gds")?;
//
//         Ok(())
//     }
// }
