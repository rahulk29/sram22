use std::path::PathBuf;

pub struct Testbench {
    name: Option<String>,
    includes: Vec<PathBuf>,
    libs: Vec<SpiceLib>,
    source: NetlistSource,
}

pub(crate) struct SpiceLib {
    pub(crate) path: PathBuf,
    pub(crate) name: Option<String>,
}

pub enum NetlistSource {
    Str(String),
    File(PathBuf),
}

impl Testbench {
    pub fn with_source(source: NetlistSource) -> Self {
        Self {
            name: None,
            includes: vec![],
            libs: vec![],
            source,
        }
    }

    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[inline]
    pub fn includes(&self) -> &[PathBuf] {
        &self.includes
    }

    #[inline]
    pub fn include(&mut self, path: PathBuf) -> &mut Self {
        self.includes.push(path);
        self
    }

    #[inline]
    pub fn add_lib(&mut self, path: PathBuf) -> &mut Self {
        self.libs.push(SpiceLib { path, name: None });
        self
    }

    #[inline]
    pub fn add_named_lib(&mut self, path: PathBuf, name: String) -> &mut Self {
        self.libs.push(SpiceLib {
            path,
            name: Some(name),
        });
        self
    }

    #[inline]
    pub(crate) fn libs(&self) -> &[SpiceLib] {
        &self.libs
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
