use std::path::PathBuf;

use crate::BUILD_PATH;

pub(crate) fn test_work_dir(name: &str) -> PathBuf {
    PathBuf::from(BUILD_PATH).join(name)
}
