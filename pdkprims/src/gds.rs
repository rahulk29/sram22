use std::{path::Path, collections::HashMap};

use layout21::{utils::Ptr, raw::{Cell, Library}, gds21::GdsLibrary};

use crate::PdkLib;

fn name_map(lib: &Library) -> HashMap<String, Ptr<Cell>> {
    let mut map = HashMap::with_capacity(lib.cells.len());

    for cell in lib.cells.iter() {
        let icell = cell.read().unwrap();
        map.insert(icell.name.clone(), Ptr::clone(cell));
    }

    map
}

pub fn cell_gds(
    pdk_lib: &mut PdkLib,
    gds_file: impl AsRef<Path>,
    cell_name: &str,
) -> anyhow::Result<Ptr<Cell>> {
    let lib = GdsLibrary::load(gds_file)?;
    let lib = Library::from_gds(&lib, Some(pdk_lib.pdk.layers.clone()))?;

    let map = name_map(&pdk_lib.lib);

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

    let mut t_cell = None;

    for cell in lib.cells.iter() {
        let inner = cell.read().unwrap();
        if inner.name == cell_name {
            t_cell = Some(cell);
        }

        let mut flag = false;

        for ecell in pdk_lib.lib.cells.iter() {
            let ecell = ecell.read().unwrap();
            if ecell.name == inner.name {
                flag = true;
                break;
            }
        }

        if !flag {
            pdk_lib.lib.cells.push(cell.clone());
        }
    }

    Ok(t_cell.map(Ptr::clone).unwrap())
}
