use crate::config::SramConfig;

use crate::plan::extract::ExtractionResult;
use crate::plan::{execute_plan, generate_plan};
use crate::{Result, BUILD_PATH};

use std::path::PathBuf;

mod bitcells;
mod col_inv;
mod control;
mod decoder;
mod dff;
mod dout_buffer;
mod edge_detector;
mod gate;
mod guard_ring;
mod inv_chain;
mod latch;
mod mux;
mod precharge;
mod rbl;
mod sense_amp;
mod sram;
mod tmc;
mod wl_driver;
mod wmask_control;

pub(crate) fn test_work_dir(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(name)
}

pub(crate) fn generate_test(config: &SramConfig) -> Result<()> {
    let plan = generate_plan(ExtractionResult {}, config)?;
    let name = &plan.sram_params.name;

    let work_dir = test_work_dir(name);
    execute_plan(&work_dir, &plan)?;

    #[cfg(feature = "calibre")]
    {
        crate::verification::calibre::run_sram_drc(&work_dir, name)?;
        crate::verification::calibre::run_sram_lvs(&work_dir, name, plan.sram_params.control)?;
        #[cfg(feature = "pex")]
        crate::verification::calibre::run_sram_pex(&work_dir, name, plan.sram_params.control)?;
    }

    #[cfg(feature = "spectre")]
    crate::verification::spectre::run_sram_spectre(&plan.sram_params, &work_dir, name)?;

    Ok(())
}
