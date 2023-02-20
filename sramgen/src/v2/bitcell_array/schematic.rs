use substrate::component::NoParams;
use substrate::index::IndexOwned;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;

use super::SpCellArray;
use crate::v2::macros::SpCell;

impl SpCellArray {
    pub(crate) fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);
        let bl = ctx.bus_port("bl", self.params.cols, Direction::InOut);
        let br = ctx.bus_port("br", self.params.cols, Direction::InOut);
        let wl = ctx.bus_port("wl", self.params.rows, Direction::Input);

        let make_cell =
            |ctx: &mut SchematicCtx, wl, bl, br, name| -> substrate::error::Result<()> {
                let mut cell = ctx.instantiate::<SpCell>(&NoParams)?;
                cell.connect_all([
                    ("BL", bl),
                    ("BR", br),
                    ("VDD", vdd),
                    ("VSS", vss),
                    ("WL", wl),
                    ("VNB", vss),
                    ("VPB", vdd),
                ]);
                cell.set_name(name);
                ctx.add_instance(cell);
                Ok(())
            };

        for i in 0..self.params.rows {
            for j in 0..self.params.cols {
                // .subckt sky130_fd_bd_sram__sram_sp_cell_opt1a BL BR VDD VSS WL VNB VPB
                make_cell(
                    ctx,
                    wl.index(i),
                    bl.index(j),
                    br.index(j),
                    arcstr::format!("cell_{i}_{j}"),
                )?;
            }
        }

        for i in 0..self.params.rows + 2 {
            let wl = if i == 0 || i == self.params.rows + 1 {
                vss
            } else {
                wl.index(i - 1)
            };
            make_cell(ctx, wl, vdd, vdd, arcstr::format!("dummy_col_left_{i}"))?;
            make_cell(ctx, wl, vdd, vdd, arcstr::format!("dummy_col_right_{i}"))?;
        }

        for j in 0..self.params.cols {
            let bl = bl.index(j);
            let br = br.index(j);
            make_cell(ctx, vss, bl, br, arcstr::format!("dummy_row_top_{j}"))?;
            make_cell(ctx, vss, bl, br, arcstr::format!("dummy_row_bot_{j}"))?;
        }
        Ok(())
    }
}
