use std::path::PathBuf;

pub struct Testbench {
    name: Option<String>,
    includes: Vec<PathBuf>,
    libs: Vec<SpiceLib>,
    source: NetlistSource,
}

pub struct SpiceLib {
    path: PathBuf,
    name: Option<String>,
}

pub enum NetlistSource {
    Str(String),
    File(PathBuf),
}

impl Testbench {
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[inline]
    pub fn source(&self) -> &NetlistSource {
        &self.source
    }
}
