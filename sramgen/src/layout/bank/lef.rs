use layout21::lef21::{
    LefDbuPerMicron, LefDecimal, LefGeometry, LefLayerGeometriesBuilder, LefLibrary,
    LefLibraryBuilder, LefMacroBuilder, LefPin, LefPinDirection, LefPinUse, LefPoint,
    LefPortBuilder, LefPortClass, LefShape, LefSymmetry, LefUnits,
};
use layout21::raw::{BoundBoxTrait, Cell, Instance, Point, Rect};
use layout21::utils::Ptr;
use pdkprims::Pdk;

use crate::layout::power::{PowerSource, PowerStraps};

pub const DB_PER_MICRON: u32 = 1000;

pub struct Params<'a> {
    pub addr_bits: usize,
    pub data_bits: usize,
    pub cell: Ptr<Cell>,
    pub straps: &'a PowerStraps,
    pub pdk: Pdk,
}

pub fn generate(params: Params<'_>) -> LefLibrary {
    let Params {
        addr_bits,
        data_bits,
        cell,
        straps,
        pdk,
    } = params;
    let inst = Instance::new("", cell);

    let mut pins = Vec::new();

    let m2 = pdk.metal(2);
    let m3 = pdk.metal(3);

    // Address bits
    for i in 0..addr_bits {
        let pin_name = format!("addr_{i}");
        let rect = inst.port(pin_name).largest_rect(m2).unwrap();

        pins.push(export_pin(ExportPin {
            layer_name: "met2",
            pin_name: format!("addr[{i}]"),
            rects: &[rect],
            direction: LefPinDirection::Input,
        }));
    }

    // Write enable
    let rect = inst.port("we").largest_rect(m2).unwrap();
    pins.push(export_pin(ExportPin {
        layer_name: "met2",
        pin_name: "we".to_string(),
        rects: &[rect],
        direction: LefPinDirection::Input,
    }));

    // Data bits
    for i in 0..data_bits {
        let rect = inst.port(format!("dout_{i}")).largest_rect(m3).unwrap();
        pins.push(export_pin(ExportPin {
            layer_name: "met3",
            pin_name: format!("dout[{i}]"),
            rects: &[rect],
            direction: LefPinDirection::Output { tristate: false },
        }));

        let rect = inst.port(format!("din_{i}")).largest_rect(m3).unwrap();
        pins.push(export_pin(ExportPin {
            layer_name: "met3",
            pin_name: format!("din[{i}]"),
            rects: &[rect],
            direction: LefPinDirection::Input,
        }));
    }

    // Power
    let vdd = straps
        .v_traces
        .iter()
        .filter(|(src, _rect)| *src == PowerSource::Vdd)
        .map(|(_, rect)| *rect)
        .collect::<Vec<_>>();
    let vss = straps
        .v_traces
        .iter()
        .filter(|(src, _rect)| *src == PowerSource::Gnd)
        .map(|(_, rect)| *rect)
        .collect::<Vec<_>>();

    pins.push(export_pin(ExportPin {
        layer_name: "met3",
        pin_name: "vdd".to_string(),
        rects: &vdd,
        direction: LefPinDirection::Inout,
    }));
    pins.push(export_pin(ExportPin {
        layer_name: "met3",
        pin_name: "vss".to_string(),
        rects: &vss,
        direction: LefPinDirection::Inout,
    }));

    let bbox = inst.bbox().into_rect();

    // macro is a reserved keyword in Rust
    let makro = LefMacroBuilder::default()
        .name("sram_bank")
        .pins(pins)
        .size((export_decimal(bbox.width()), export_decimal(bbox.height())))
        .symmetry([LefSymmetry::X, LefSymmetry::Y, LefSymmetry::R90])
        .build()
        .unwrap();

    let units = LefUnits {
        database_microns: Some(LefDbuPerMicron(DB_PER_MICRON)),
        ..Default::default()
    };

    LefLibraryBuilder::default()
        .macros([makro])
        .bus_bit_chars(('[', ']'))
        .divider_char('/')
        .units(units)
        .vias(layout21::lef21::Unsupported)
        .sites([])
        .build()
        .unwrap()
}

fn export_rect(r: Rect) -> LefShape {
    LefShape::Rect(export_point(r.p0), export_point(r.p1))
}

fn export_point(p: Point) -> LefPoint {
    LefPoint::new(export_decimal(p.x), export_decimal(p.y))
}

fn export_decimal(x: isize) -> LefDecimal {
    LefDecimal::new(x as i64, 3)
}

struct ExportPin<'a> {
    layer_name: &'a str,
    pin_name: String,
    rects: &'a [Rect],
    direction: LefPinDirection,
}

fn export_pin(pin_info: ExportPin<'_>) -> LefPin {
    let ExportPin {
        layer_name,
        pin_name,
        rects,
        direction,
    } = pin_info;
    let geometries = rects
        .iter()
        .map(|rect| {
            LefLayerGeometriesBuilder::default()
                .layer_name(layer_name)
                .geometries([LefGeometry::Shape(export_rect(*rect))])
                .vias([])
                .build()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let port = LefPortBuilder::default()
        .class(LefPortClass::None)
        .layers(geometries)
        .build()
        .unwrap();

    LefPin {
        name: pin_name,
        ports: vec![port],
        direction: Some(direction),
        use_: Some(LefPinUse::Signal),
        ..Default::default()
    }
}
