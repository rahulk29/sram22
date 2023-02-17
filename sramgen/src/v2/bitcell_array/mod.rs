use std::path::PathBuf;

use arcstr::ArcStr;
use codegen::hard_macro;
use serde::{Deserialize, Serialize};
use substrate::component::{Component, View};
use substrate::data::SubstrateCtx;

use crate::tech::{external_gds_path, external_spice_path};

mod cbl;
mod layout;
mod replica;
mod schematic;

pub struct SpCellArray {
    params: SpCellArrayParams,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct SpCellArrayParams {
    pub rows: usize,
    pub cols: usize,
    pub mux_ratio: usize,
}

impl Component for SpCellArray {
    type Params = SpCellArrayParams;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        if params.rows % 8 != 0 || params.cols % 8 != 0 || params.rows == 0 || params.cols == 0 {
            return Err(substrate::component::error::Error::InvalidParams.into());
        }
        Ok(Self { params: *params })
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("sp_cell_array")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        self.schematic(ctx)
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        self.layout(ctx)
    }
}

#[cfg(test)]
mod tests {
    use substrate::component::{Component, NoParams};
    use substrate::layout::geom::Rect;
    use substrate::layout::layers::selector::Selector;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    use crate::v2::bitcell_array::layout::*;
    use crate::v2::guard_ring::{GuardRingParams, GuardRingWrapper, WrapperParams};

    use super::*;

    pub struct SpCellArrayWithGuardRing {
        params: WrapperParams<SpCellArrayParams>,
    }

    impl Component for SpCellArrayWithGuardRing {
        type Params = WrapperParams<SpCellArrayParams>;

        fn new(
            params: &Self::Params,
            _ctx: &substrate::data::SubstrateCtx,
        ) -> substrate::error::Result<Self> {
            Ok(Self {
                params: params.clone(),
            })
        }

        fn name(&self) -> ArcStr {
            arcstr::literal!("sp_cell_array")
        }

        fn schematic(
            &self,
            ctx: &mut substrate::schematic::context::SchematicCtx,
        ) -> substrate::error::Result<()> {
            let array = ctx.instantiate::<SpCellArray>(&self.params.inner)?;
            ctx.add_instance(array);
            Ok(())
        }

        fn layout(
            &self,
            ctx: &mut substrate::layout::context::LayoutCtx,
        ) -> substrate::error::Result<()> {
            let layers = ctx.layers();
            let m1 = layers.get(Selector::Metal(1))?;
            let m2 = layers.get(Selector::Metal(2))?;
            let params = WrapperParams {
                inner: SpCellArrayParams {
                    rows: 32,
                    cols: 32,
                    mux_ratio: 4,
                },
                ring: GuardRingParams {
                    enclosure: Rect::default(),
                    h_metal: m2,
                    v_metal: m1,
                    h_width: 1_360,
                    v_width: 1_360,
                },
            };
            let array = ctx.instantiate::<GuardRingWrapper<SpCellArray>>(&params)?;
            ctx.draw(array)?;
            self.layout(ctx)
        }
    }

    #[test]
    fn test_sp_cell_array() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array");
        let params = SpCellArrayParams {
            rows: 32,
            cols: 32,
            mux_ratio: 4,
        };
        ctx.write_layout::<SpCellArray>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");

        ctx.write_schematic_to_file::<SpCellArray>(&params, out_spice(&work_dir, "schematic"))
            .expect("failed to write schematic");
    }

    #[test]
    fn test_sp_cell_array_with_guard_ring() -> substrate::error::Result<()> {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array_with_guard_ring");

        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let params = WrapperParams {
            inner: SpCellArrayParams {
                rows: 32,
                cols: 32,
                mux_ratio: 4,
            },
            ring: GuardRingParams {
                enclosure: Rect::default(),
                h_metal: m2,
                v_metal: m1,
                h_width: 1_360,
                v_width: 1_360,
            },
        };
        ctx.write_layout::<GuardRingWrapper<SpCellArray>>(&params, out_gds(&work_dir, "layout"))?;
        Ok(())
    }

    #[test]
    fn test_sp_cell_array_tiles() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sp_cell_array_tiles");
        let tap_ratio = TapRatio {
            mux_ratio: 4,
            hstrap_ratio: 8,
        };
        ctx.write_layout::<SpCellArrayCornerUl>(&NoParams, out_gds(&work_dir, "corner_ul"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCornerUr>(&NoParams, out_gds(&work_dir, "corner_ur"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCornerLr>(&NoParams, out_gds(&work_dir, "corner_lr"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCornerLl>(&NoParams, out_gds(&work_dir, "corner_ll"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayBottom>(&tap_ratio, out_gds(&work_dir, "bottom"))
            .expect("failed to write layout");
        ctx.write_layout::<SpCellArrayCenter>(&tap_ratio, out_gds(&work_dir, "center"))
            .expect("failed to write layout");
    }

    #[cfg(feature = "calibre")]
    #[test]
    fn test_dff_lvs_pex() {}
}
