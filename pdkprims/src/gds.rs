use std::{collections::HashMap, path::Path};

use layout21::{
    gds21::GdsLibrary,
    raw::{Cell, Library},
    utils::Ptr,
};

use crate::PdkLib;

fn name_map(lib: &Library) -> HashMap<String, Ptr<Cell>> {
    let mut map = HashMap::with_capacity(lib.cells.len());

    for cell in lib.cells.iter() {
        let icell = cell.read().unwrap();
        map.insert(icell.name.clone(), Ptr::clone(cell));
    }

    map
}

/// Loads GDS file `gds_file` into the given library.
pub fn load_gds(pdk_lib: &mut PdkLib, gds_file: impl AsRef<Path>) -> anyhow::Result<()> {
    let lib = GdsLibrary::load(gds_file)?;
    let lib = Library::from_gds(&lib, Some(pdk_lib.pdk.layers.clone()))?;

    let mut map = name_map(&pdk_lib.lib);

    for cell in lib.cells.iter() {
        let mut inner = cell.write().unwrap();
        if let Some(ref mut lay) = inner.layout {
            for inst in lay.insts.iter_mut() {
                let remap_cell = {
                    let icell = inst.cell.read().unwrap();
                    if let Some(ncell) = map.get(&icell.name) {
                        Ptr::clone(ncell)
                    } else {
                        Ptr::clone(&inst.cell)
                    }
                };
                inst.cell = remap_cell;
            }
        }
    }

    // Do not add cells that already exist.
    for cell in lib.cells.iter() {
        let new_cell = cell.read().unwrap();

        if !map.contains_key(&new_cell.name) {
            pdk_lib.lib.cells.push(Ptr::clone(cell));
            map.insert(new_cell.name.clone(), Ptr::clone(cell));
        }
    }

    Ok(())
}
