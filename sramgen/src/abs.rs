use crate::{Result, BUILD_PATH};
use abstract_lef::{run_abstract, AbstractParams};
use std::path::{Path, PathBuf};

pub fn run_sram_abstract(
    name: &str,
    lef_path: impl AsRef<Path>,
    gds_path: impl AsRef<Path>,
    verilog_path: impl AsRef<Path>,
) -> Result<()> {
    let work_dir = PathBuf::from(BUILD_PATH).join(format!("lef/{}", name));

    run_abstract(AbstractParams {
        work_dir: &work_dir,
        cell_name: name,
        gds_path: gds_path.as_ref(),
        verilog_path: verilog_path.as_ref(),
        lef_path: lef_path.as_ref(),
    })?;

    Ok(())
}
