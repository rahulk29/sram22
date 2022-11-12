use std::sync::Arc;

use crate::layout::Result;
use crate::tech::openram_dff_gds;
use derive_builder::Builder;
use layout21::raw::align::AlignRect;
use layout21::raw::translate::Translate;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Dir, Element, Instance, Int, Layout, Point, Shape,
};
use layout21::utils::Ptr;
use pdkprims::contact::Contact;
use pdkprims::PdkLib;

use crate::layout::array::*;

use super::bank::GateList;
use super::common::{GridOrder, MergeArgs};

pub fn draw_dff_array(
    lib: &mut PdkLib,
    name: impl Into<String>,
    width: usize,
) -> Result<ArrayedCell> {
    let dff = openram_dff_gds(lib)?;

    draw_cell_array(
        ArrayCellParams {
            name: name.into(),
            num: width,
            cell: dff,
            spacing: None,
            flip: FlipMode::None,
            flip_toggle: false,
            direction: Dir::Horiz,
        },
        lib,
    )
}

pub fn draw_vert_dff_array(
    lib: &mut PdkLib,
    name: impl Into<String>,
    width: usize,
) -> Result<Ptr<Cell>> {
    let name = name.into();

    let mut layout = Layout::new(name.clone());
    let mut abs = Abstract::new(name.clone());

    let mut prev: Option<Instance> = None;

    let dff = openram_dff_gds(lib)?;

    let m0 = lib.pdk.metal(0);

    for i in 0..width {
        let mut inst = Instance::new(format!("dff_{}", i), dff.clone());
        if i % 2 == 0 {
            inst.reflect_vert = true;
        }

        let port = if i % 2 == 0 { "vdd" } else { "gnd" };

        if let Some(prev) = prev {
            let new_bot = inst.port(port).largest_rect(m0).unwrap().p0.y;
            let prev_bot = prev.port(port).largest_rect(m0).unwrap().p0.y;
            inst.translate(Point::new(0, prev_bot - new_bot));
        }

        let mut ports = inst.ports();
        for p in ports.iter_mut() {
            p.net = format!("{}_{}", &p.net, i);
        }
        for port in ports {
            abs.add_port(port);
        }

        layout.add_inst(inst.clone());
        prev = Some(inst);
    }

    let ptr = Ptr::new(Cell {
        name,
        layout: Some(layout),
        abs: Some(abs),
    });

    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[derive(Clone, Eq, PartialEq, Builder)]
pub struct DffGridParams {
    #[builder(setter(into))]
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    #[builder(setter(strip_option), default)]
    pub row_pitch: Option<Int>,
    #[builder(default = "GridOrder::ColumnMajor")]
    pub order: GridOrder,
}

impl DffGridParams {
    #[inline]
    pub fn builder() -> DffGridParamsBuilder {
        DffGridParamsBuilder::default()
    }
}

pub fn draw_dff_grid(lib: &mut PdkLib, params: DffGridParams) -> Result<Ptr<Cell>> {
    let DffGridParams {
        name,
        rows,
        cols,
        row_pitch,
        order,
    } = params;

    let mut cell = Cell::empty(name);

    let dff = openram_dff_gds(lib)?;
    let mut tmp = Instance::new("", dff.clone());
    let m0 = lib.pdk.metal(0);
    let m1 = lib.pdk.metal(1);
    let m2 = lib.pdk.metal(2);
    let vdd = tmp.port("vdd").largest_rect(m0).unwrap();
    let vss = tmp.port("gnd").largest_rect(m0).unwrap();

    tmp.reflect_vert = true;
    let vss_flipped = tmp.port("gnd").largest_rect(m0).unwrap();
    let y_offset_flip = vss.top() - vss_flipped.top();
    let y_offset = vdd.top() - vss.top();

    let horiz_pitch = row_pitch.unwrap_or_else(|| vdd.width());
    let mut tap_cell: Option<(Arc<Contact>, Arc<Contact>)> = None;

    for j in 0..rows {
        let mut row_dffs = Vec::with_capacity(cols);
        for i in 0..cols {
            let mut inst = Instance::new(format!("dff_{}_{}", i, j), dff.clone());
            inst.loc.x = (i as isize) * horiz_pitch;
            let ji = j as isize;
            inst.loc.y = -((ji / 2) * 2 * y_offset + (ji % 2) * y_offset_flip);
            inst.reflect_vert = (ji % 2) == 1;

            let port_idx = match order {
                GridOrder::RowMajor => i + j * cols,
                GridOrder::ColumnMajor => j + i * rows,
            };

            for mut port in inst.ports() {
                if port.net == "d" || port.net == "q" || port.net == "qn" || port.net == "clk" {
                    port.net = format!("{}_{}", &port.net, port_idx);
                    cell.abs_mut().add_port(port);
                }
            }

            cell.layout_mut().add_inst(inst.clone());
            row_dffs.push(inst);
        }

        for port in ["vdd", "gnd", "vpb"] {
            let layer = if port == "vpb" {
                lib.pdk.get_layerkey("nwell").unwrap()
            } else {
                m0
            };
            let rect = MergeArgs::builder()
                .layer(layer)
                .insts(GateList::Cells(&row_dffs))
                .port_name(port)
                .build()?
                .rect();
            if port == "vdd" || port == "gnd" {
                let ct_boundary = rect.expand(170 / 2);
                let (c1, c2) = match tap_cell {
                    Some((ref c1, ref c2)) => (c1.clone(), c2.clone()),
                    None => {
                        let c1 = lib
                            .pdk
                            .get_contact_within("viali", m0, ct_boundary)
                            .unwrap();
                        let c2 = lib.pdk.get_contact_within("via1", m1, ct_boundary).unwrap();
                        tap_cell = Some((c1.clone(), c2.clone()));
                        (c1, c2)
                    }
                };

                let mut i1 = Instance::new(format!("licon_{j}"), c1.cell.clone());
                let mut i2 = Instance::new(format!("via1_{j}"), c2.cell.clone());

                i1.align_centers_gridded(rect.bbox(), lib.pdk.grid());
                i2.align_centers_gridded(rect.bbox(), lib.pdk.grid());

                let port_rect = i2.port("x").largest_rect(m2).unwrap();
                cell.layout_mut().add_inst(i1);
                cell.layout_mut().add_inst(i2);

                let mut port = AbstractPort::new(format!("{}_{}", port, j));
                port.add_shape(m2, Shape::Rect(port_rect));
                cell.abs_mut().add_port(port);
            }

            cell.layout_mut().add(Element {
                net: None,
                layer,
                purpose: layout21::raw::LayerPurpose::Drawing,
                inner: Shape::Rect(rect),
            });
        }
    }

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::tech::COLUMN_WIDTH;
    use crate::utils::test_gds_path;

    use super::*;

    #[test]
    fn test_sky130_dff_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_dff_array")?;
        draw_dff_array(&mut lib, "test_sky130_dff_array", 16)?;

        lib.save_gds(test_gds_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_vert_dff_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_vert_dff_array")?;
        draw_vert_dff_array(&mut lib, "test_sky130_vert_dff_array", 8)?;

        lib.save_gds(test_gds_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_dff_grid() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_dff_grid")?;
        let params = DffGridParams::builder()
            .name("test_sky130_dff_grid")
            .rows(4)
            .cols(8)
            .row_pitch(4 * COLUMN_WIDTH)
            .build()?;
        draw_dff_grid(&mut lib, params)?;

        lib.save_gds(test_gds_path(&lib))?;

        Ok(())
    }
}
