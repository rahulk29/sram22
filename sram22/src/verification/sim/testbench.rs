use std::path::PathBuf;

pub struct Testbench {
    name: Option<String>,
    // includes: Vec<PathBuf>,
    // libs: Vec<SpiceLib>,
    source: NetlistSource,
}

pub struct SpiceLib {
    // path: PathBuf,
// name: Option<String>,
}

pub enum NetlistSource {
    Str(String),
    File(PathBuf),
}

impl Testbench {
    pub fn with_source(source: NetlistSource) -> Self {
        Self { name: None, source }
    }

    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[inline]
    pub fn source(&self) -> &NetlistSource {
        &self.source
    }

    #[inline]
    pub fn set_source(&mut self, source: NetlistSource) -> &mut Self {
        self.source = source;
        self
    }

    #[inline]
    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }
}
