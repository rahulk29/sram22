use crate::Result;
use abstract_lef::AbstractParams;
use std::path::{Path, PathBuf};

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
