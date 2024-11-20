use crate::blocks::columns;
use crate::blocks::sram::{Sram, SramParams};
use crate::Result;
use abstract_lef::AbstractParams;
use lef21::{
    LefDecimal, LefForeign, LefGeometry, LefLayerGeometries, LefLibrary, LefMacro, LefMacroClass,
    LefPin, LefPinAntennaAttr, LefPinDirection, LefPinUse, LefPoint, LefPort, LefShape,
    LefSymmetry,
};
use std::path::{Path, PathBuf};
use subgeom::bbox::BoundBox;
use subgeom::{Corner, Point};
use substrate::data::SubstrateCtx;
use substrate::layout::cell::{Port, PortId};
use substrate::layout::layers::selector::Selector;

pub fn run_abstract(
    work_dir: impl AsRef<Path>,
    name: &str,
    lef_path: impl AsRef<Path>,
    gds_path: impl AsRef<Path>,
    verilog_path: impl AsRef<Path>,
) -> Result<()> {
    let abs_work_dir = PathBuf::from(work_dir.as_ref()).join("lef");

    abstract_lef::run_abstract(AbstractParams {
        work_dir: &abs_work_dir,
        cell_name: name,
        gds_path: gds_path.as_ref(),
        verilog_path: verilog_path.as_ref(),
        lef_path: lef_path.as_ref(),
    })?;

    Ok(())
}

pub fn write_abstract(
    ctx: &SubstrateCtx,
    params: &SramParams,
    path: impl AsRef<Path>,
) -> Result<()> {
    let sram = ctx.instantiate_layout::<Sram>(params)?;
    let m1 = ctx.layers().get(Selector::Metal(1))?;
    let m2 = ctx.layers().get(Selector::Metal(2))?;
    let mut lef = LefLibrary::new();
    lef.version = Some(LefDecimal::new(58, 1));
    lef.bus_bit_chars = Some(('[', ']'));
    lef.divider_char = Some('/');
    let mut lef_macro = LefMacro::new(params.name().as_str());
    let brect = sram.brect();
    let ll = brect.corner(Corner::LowerLeft);
    lef_macro.class = Some(LefMacroClass::Block { tp: None });
    lef_macro.origin = (ll != Point::zero())
        .then(|| LefPoint::new(LefDecimal::new(-ll.x, 3), LefDecimal::new(-ll.y, 3)));
    lef_macro.foreign = Some(LefForeign {
        cell_name: params.name().as_str().to_string(),
        pt: (ll != Point::zero())
            .then(|| LefPoint::new(LefDecimal::new(ll.x, 3), LefDecimal::new(ll.y, 3))),
        orient: None,
    });
    lef_macro.size = Some((
        LefDecimal::new(brect.width(), 3),
        LefDecimal::new(brect.height(), 3),
    ));
    lef_macro.symmetry = Some(vec![LefSymmetry::X, LefSymmetry::Y, LefSymmetry::R90]);
    for layer_name in ["met1", "met2"] {
        lef_macro.obs.push(LefLayerGeometries {
            layer_name: layer_name.to_string(),
            geometries: vec![LefGeometry::Shape(LefShape::Rect(
                None,
                LefPoint::new(
                    LefDecimal::new(brect.p0.x, 3),
                    LefDecimal::new(brect.p0.y, 3),
                ),
                LefPoint::new(
                    LefDecimal::new(brect.p1.x, 3),
                    LefDecimal::new(brect.p1.y, 3),
                ),
            ))],
            vias: Vec::new(),
            except_pg_net: None,
            spacing: None,
            width: None,
        });
    }

    let get_layer_name = |layer_key| {
        if layer_key == m1 {
            "met1"
        } else if layer_key == m2 {
            "met2"
        } else {
            unimplemented!()
        }
    };

    for (pin, layer, width, direction) in [
        (
            "dout",
            m1,
            params.data_width(),
            LefPinDirection::Output { tristate: false },
        ),
        ("din", m1, params.data_width(), LefPinDirection::Input),
        ("wmask", m1, params.wmask_width(), LefPinDirection::Input),
        ("addr", m1, params.addr_width(), LefPinDirection::Input),
        ("we", m1, 1, LefPinDirection::Input),
        ("ce", m1, 1, LefPinDirection::Input),
        ("clk", m1, 1, LefPinDirection::Input),
        ("rstb", m1, 1, LefPinDirection::Input),
        ("vdd", m2, 1, LefPinDirection::Inout),
        ("vss", m2, 1, LefPinDirection::Inout),
    ] {
        for i in 0..width {
            lef_macro.pins.push(LefPin {
                name: if width > 1 {
                    format!("{pin}[{i}]")
                } else {
                    pin.to_string()
                },
                ports: vec![LefPort {
                    class: None,
                    layers: vec![LefLayerGeometries {
                        layer_name: get_layer_name(layer).to_string(),
                        geometries: sram
                            .port(PortId::new(pin, i))?
                            .shapes(layer)
                            .filter_map(|shape| shape.as_rect())
                            .map(|rect| match pin {
                                "vdd" | "vss" => LefGeometry::Shape(LefShape::Rect(
                                    None,
                                    LefPoint::new(
                                        LefDecimal::new(rect.left(), 3),
                                        LefDecimal::new(rect.bottom(), 3),
                                    ),
                                    LefPoint::new(
                                        LefDecimal::new(rect.right(), 3),
                                        LefDecimal::new(rect.top(), 3),
                                    ),
                                )),
                                _ => LefGeometry::Shape(LefShape::Rect(
                                    None,
                                    LefPoint::new(
                                        LefDecimal::new(rect.left(), 3),
                                        LefDecimal::new(rect.bottom(), 3),
                                    ),
                                    LefPoint::new(
                                        LefDecimal::new(rect.right(), 3),
                                        LefDecimal::new(rect.bottom() + rect.width(), 3),
                                    ),
                                )),
                            })
                            .collect(),
                        vias: Vec::new(),
                        except_pg_net: None,
                        spacing: None,
                        width: None,
                    }],
                }],
                direction: Some(direction.clone()),
                use_: match pin {
                    "vdd" => Some(LefPinUse::Power),
                    "vss" => Some(LefPinUse::Ground),
                    _ => None,
                },
                antenna_model: None,
                antenna_attrs: match pin {
                    "addr" | "ce" | "we" => vec![
                        LefPinAntennaAttr {
                            key: "ANTENNAGATEAREA".to_string(),
                            val: LefDecimal::new(126_000, 6),
                            layer: Some("met1".to_string()),
                        },
                        LefPinAntennaAttr {
                            key: "ANTENNAPARTIALMETALAREA".to_string(),
                            // Conservative estimate of extra m1
                            val: LefDecimal::new(
                                149_500 + sram.port(PortId::new(pin, i))?.largest_rect(m1)?.area(),
                                6,
                            ),
                            // due to via.
                            layer: Some("met1".to_string()),
                        },
                    ],
                    "wmask" => vec![
                        LefPinAntennaAttr {
                            key: "ANTENNAGATEAREA".to_string(),
                            // Conservative estimate of extra m1
                            val: LefDecimal::new(126_000, 6),
                            // due to via.
                            layer: Some("met2".to_string()),
                        },
                        LefPinAntennaAttr {
                            key: "ANTENNAPARTIALMETALAREA".to_string(),
                            // Conservative estimate of extra m1
                            val: LefDecimal::new(
                                149_500 + sram.port(PortId::new(pin, i))?.largest_rect(m1)?.area(),
                                6,
                            ),
                            // due to via.
                            layer: Some("met1".to_string()),
                        },
                        LefPinAntennaAttr {
                            key: "ANTENNAPARTIALMETALAREA".to_string(),
                            // Conservative estimate of extra m2 from vias.
                            val: LefDecimal::new(800 * 140 + 2 * 83_200, 6),
                            // due to via.
                            layer: Some("met2".to_string()),
                        },
                    ],
                    "din" => vec![
                        LefPinAntennaAttr {
                            key: "ANTENNAGATEAREA".to_string(),
                            val: LefDecimal::new(126_000, 6),
                            layer: Some("met2".to_string()),
                        },
                        LefPinAntennaAttr {
                            key: "ANTENNAPARTIALMETALAREA".to_string(),
                            // Conservative estimate of extra m1 due to via.
                            val: LefDecimal::new(
                                149_500 + sram.port(PortId::new(pin, i))?.largest_rect(m1)?.area(),
                                6,
                            ),
                            layer: Some("met1".to_string()),
                        },
                        LefPinAntennaAttr {
                            key: "ANTENNAPARTIALMETALAREA".to_string(),
                            val: LefDecimal::new(
                                sram.cell()
                                    .get_metadata::<columns::layout::Metadata>()
                                    .dout_din_m2_area,
                                6,
                            ),
                            layer: Some("met2".to_string()),
                        },
                    ],
                    "dout" => vec![
                        LefPinAntennaAttr {
                            key: "ANTENNADIFFAREA".to_string(),
                            val: LefDecimal::new(
                                sram.cell()
                                    .get_metadata::<columns::layout::Metadata>()
                                    .dout_diff_area,
                                6,
                            ),
                            layer: Some("met2".to_string()),
                        },
                        LefPinAntennaAttr {
                            key: "ANTENNAPARTIALMETALAREA".to_string(),
                            // Conservative estimate of extra m1 due to via.
                            val: LefDecimal::new(
                                149_500 + sram.port(PortId::new(pin, i))?.largest_rect(m1)?.area(),
                                6,
                            ),
                            layer: Some("met1".to_string()),
                        },
                        LefPinAntennaAttr {
                            key: "ANTENNAPARTIALMETALAREA".to_string(),
                            val: LefDecimal::new(
                                sram.cell()
                                    .get_metadata::<columns::layout::Metadata>()
                                    .dout_din_m2_area
                                    + 800 * 140
                                    + 2 * 83_200,
                                6,
                            ),
                            layer: Some("met2".to_string()),
                        },
                    ],
                    "clk" => vec![
                        LefPinAntennaAttr {
                            key: "ANTENNAGATEAREA".to_string(),
                            val: LefDecimal::new(
                                279_000
                                    * (params.addr_width()
                                        + 2
                                        + 2 * params.data_width()
                                        + params.wmask_width())
                                        as i64
                                    + 558_000,
                                6,
                            ),
                            layer: Some("met2".to_string()),
                        },
                        // LefPinAntennaAttr {
                        //     key: "ANTENNAPARTIALMETALAREA".to_string(),
                        //     val: LefDecimal::new(
                        //         sram.port(PortId::new(pin, i))?.largest_rect(m1)?.area(),
                        //         6,
                        //     ), // TODO: Correctly calculate this area.
                        //     layer: Some("met1".to_string()),
                        // },
                    ],
                    "rstb" => vec![
                        LefPinAntennaAttr {
                            key: "ANTENNAGATEAREA".to_string(),
                            val: LefDecimal::new(
                                279_000
                                    * (params.addr_width()
                                        + 2
                                        + 2 * params.data_width()
                                        + params.wmask_width())
                                        as i64
                                    + 4_464_000,
                                6,
                            ),
                            layer: Some("met2".to_string()),
                        },
                        // LefPinAntennaAttr {
                        //     key: "ANTENNAPARTIALMETALAREA".to_string(),
                        //     val: LefDecimal::new(
                        //         sram.port(PortId::new(pin, i))?.largest_rect(m1)?.area(),
                        //         6,
                        //     ), // TODO: Correctly calculate this area.
                        //     layer: Some("met1".to_string()),
                        // },
                    ],
                    _ => vec![],
                },
                shape: None,
                taper_rule: None,
                net_expr: None,
                supply_sensitivity: None,
                ground_sensitivity: None,
                must_join: None,
                properties: None,
            });
        }
    }
    lef.macros.push(lef_macro);
    lef.save(path)
        .map_err(|err| anyhow::anyhow!("Failed to save LEF {err:?}"))?;
    Ok(())
}
