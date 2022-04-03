use std::path::{Path, PathBuf};

use layout21::raw::Library;

use crate::{
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
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: 1_400,
            length: 150,
            fingers: 2,
            intent: crate::mos::Intent::Svt,
        });

    let pdk = super::pdk()?;

    let cell = pdk.draw_sky130_mos(params)?;

    let mut lib = Library::new(
        "test_draw_sky130_mos_nand2",
        pdk.config.read().unwrap().units,
    );
    lib.layers = pdk.layers();
    lib.cells.push(cell);
    let gds = lib.to_gds()?;
    gds.save("test_draw_sky130_mos_nand2.gds")?;

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
            let cell = pdk.get_contact("polyc", i, j);
            lib.cells.push(cell);
        }
    }

    let gds = lib.to_gds()?;
    gds.save(output("test_sky130_draw_contact.gds"))?;
    Ok(())
}

fn output(name: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../_build/")
        .join(name)
}
