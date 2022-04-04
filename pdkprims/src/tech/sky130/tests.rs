use std::path::{Path, PathBuf};

use layout21::{
    raw::{DepOrder, Library},
    utils::PtrList,
};

use crate::{
    contact::ContactParams,
    geometry::CoarseDirection,
    mos::{MosDevice, MosParams, MosType},
};

#[test]
fn test_draw_sky130_mos_nand2() -> Result<(), Box<dyn std::error::Error>> {
    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(CoarseDirection::Horizontal)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: 1_000,
            length: 150,
            fingers: 2,
            intent: crate::mos::Intent::Svt,
            skip_sd_metal: vec![1],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_400,
            length: 150,
            fingers: 2,
            intent: crate::mos::Intent::Svt,
            skip_sd_metal: vec![],
        });

    let pdk = super::pdk()?;

    let cell = pdk.draw_sky130_mos(params)?;

    let mut lib = Library::new(
        "test_draw_sky130_mos_nand2",
        pdk.config.read().unwrap().units,
    );
    lib.layers = pdk.layers();
    lib.cells.push(cell);
    let cells = DepOrder::order(&lib);
    lib.cells = PtrList::from_ptrs(cells);
    let gds = lib.to_gds()?;
    gds.save(output("test_draw_sky130_mos_nand2.gds"))?;

    Ok(())
}

#[test]
fn test_sky130_draw_contact() -> Result<(), Box<dyn std::error::Error>> {
    let n = 5;

    let pdk = super::pdk()?;

    let mut lib = Library::new("test_sky130_draw_contact", pdk.config.read().unwrap().units);
    lib.layers = pdk.layers();

    for i in 1..=n {
        for j in 1..=i {
            for stack in ["ndiffc", "pdiffc", "polyc", "viali", "via1", "via2"] {
                for dir in [CoarseDirection::Vertical, CoarseDirection::Horizontal] {
                    let mut cp = ContactParams::builder();
                    let cp = cp
                        .stack(stack.to_string())
                        .rows(i)
                        .cols(j)
                        .dir(dir)
                        .build()
                        .unwrap();
                    let ct = pdk.get_contact(&cp);
                    lib.cells.push(ct.cell);
                }
            }
        }
    }

    let gds = lib.to_gds()?;
    gds.save(output("test_sky130_draw_contact.gds"))?;
    Ok(())
}

#[test]
fn test_sky130_contact_sized() -> Result<(), Box<dyn std::error::Error>> {
    let pdk = super::pdk()?;

    let diff = pdk.get_layerkey("diff").unwrap();
    let ct = pdk.get_contact_sized("ndiffc", diff, 330).unwrap();
    assert_eq!(ct.cols, 1);
    assert_eq!(ct.rows, 1);

    let ct = pdk.get_contact_sized("ndiffc", diff, 650).unwrap();
    assert_eq!(ct.cols, 2);
    assert_eq!(ct.rows, 1);

    Ok(())
}

fn output(name: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../_build/")
        .join(name)
}
