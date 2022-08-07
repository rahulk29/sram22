use std::path::{Path, PathBuf};

use layout21::raw::geom::Dir;
use layout21::{
    raw::{DepOrder, LayerPurpose, Library},
    utils::{Ptr, PtrList},
};

use crate::{
    contact::ContactParams,
    mos::{MosDevice, MosParams, MosType},
};

#[test]
fn test_draw_sky130_mos_nand2() -> Result<(), Box<dyn std::error::Error>> {
    setup()?;
    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(Dir::Horiz)
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
    lib.cells.push(Ptr::clone(&cell.cell));
    let cells = DepOrder::order(&lib);
    lib.cells = PtrList::from_ptrs(cells);
    let gds = lib.to_gds()?;
    gds.save(output("test_draw_sky130_mos_nand2.gds"))?;

    Ok(())
}

#[test]
fn test_sky130_draw_contact() -> Result<(), Box<dyn std::error::Error>> {
    setup()?;
    let n = 5;

    let pdk = super::pdk()?;

    let mut lib = Library::new("test_sky130_draw_contact", pdk.config.read().unwrap().units);
    lib.layers = pdk.layers();

    for i in 1..=n {
        for j in 1..=n {
            for stack in ["ntap", "ndiffc", "pdiffc", "polyc", "viali", "via1", "via2"] {
                for dir in [Dir::Horiz, Dir::Vert] {
                    let mut cp = ContactParams::builder();
                    let cp = cp
                        .stack(stack.to_string())
                        .rows(i)
                        .cols(j)
                        .dir(dir)
                        .build()
                        .unwrap();
                    let ct = pdk.get_contact(&cp);
                    lib.cells.push(Ptr::clone(&ct.cell));
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
    setup()?;
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LayerPair {
    purpose: LayerPurpose,
    num: i16,
}

#[test]
fn test_serialize_layer_purpose() -> Result<(), Box<dyn std::error::Error>> {
    let lp = LayerPair {
        purpose: LayerPurpose::Named("named_purpose".to_string(), 24i16),
        num: 21,
    };
    let s = serde_yaml::to_string(&lp)?;
    println!("{}", &s);

    let x: LayerPair = serde_yaml::from_str(&s)?;
    println!("{:?}", &x);
    Ok(())
}

fn output(name: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../_build/")
        .join(name)
}

fn setup() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../_build/"))?;
    Ok(())
}
