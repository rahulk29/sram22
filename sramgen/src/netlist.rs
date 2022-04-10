use std::{collections::HashMap, fs::File, path::Path};

use vlsir::{
    circuit::{Instance, Package},
    reference::To,
    spice::SimInput,
    Module, Reference,
};

pub const PRIMITIVE_DOMAIN: &'static str = "primitives";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct NetlistWriter<T>
where
    T: std::io::Write,
{
    sink: T,
    tech: String,
    modules: HashMap<String, Module>,
}

impl<T> NetlistWriter<T>
where
    T: std::io::Write,
{
    pub fn new(tech: impl Into<String>, sink: T) -> Self {
        Self {
            sink,
            tech: tech.into(),
            modules: HashMap::new(),
        }
    }

    fn populate_modules(&mut self, pkg: Package) {
        for m in pkg.modules {
            self.modules.insert(m.name.clone(), m);
        }
    }

    pub fn netlist(&mut self, input: SimInput) -> Result<()> {
        self.populate_modules(input.pkg.as_ref().unwrap().clone());

        let pkg = input.pkg.as_ref().unwrap();
        writeln!(self.sink, "* {}", pkg.domain)?;
        writeln!(self.sink, "* {}", pkg.desc)?;
        writeln!(self.sink, "")?;

        for m in &pkg.modules {
            write!(self.sink, ".subckt {}", &m.name)?;
            for port in &m.ports {
                let sig = port.signal.as_ref().unwrap();
                if sig.width > 1 {
                    for i in 0..sig.width {
                        write!(self.sink, " {}_{}", &sig.name, i)?;
                    }
                } else {
                    write!(self.sink, " {}", &sig.name)?;
                }
            }
            writeln!(self.sink, "")?;

            for inst in &m.instances {
                self.emit_instance(inst)?;
            }

            writeln!(self.sink, ".ends")?;
        }
        Ok(())
    }

    fn emit_instance(&mut self, inst: &Instance) -> Result<()> {
        let reference = inst.module.as_ref().unwrap();
        match reference.to.as_ref().unwrap() {
            To::Local(name) => self.emit_local_instance(name, inst)?,
            To::External(qn) => {
                if qn.domain == PRIMITIVE_DOMAIN {
                    self.emit_primitive(inst)?;
                } else {
                    panic!("external modules not (yet) supported");
                }
            }
        };
        Ok(())
    }

    fn emit_local_instance(&mut self, name: &str, inst: &Instance) -> Result<()> {
        let module = self.modules.get(name).unwrap();
        for p in &module.ports {
            let sig = p.signal.as_ref().unwrap();
        }
        Ok(())
    }

    fn emit_primitive(&mut self, inst: &Instance) -> Result<()> {
        Ok(())
    }

    pub fn finish(self) -> Result<T> {
        Ok(self.sink)
    }
}

impl NetlistWriter<std::fs::File> {
    pub fn to_file(tech: String, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let f = File::create(path)?;
        Ok(Self {
            sink: f,
            tech,
            modules: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use tempfile::tempfile;
    use vlsir::{circuit::Package, spice::SimInput};

    use crate::{
        gate::{Nand2Params, Size},
        mos::{ext_nmos, ext_pmos},
    };

    use super::{NetlistWriter, Result};

    #[test]
    fn test_netlist() -> Result<()> {
        let nand2 = crate::gate::nand2(Nand2Params {
            length: 150,
            size: Size {
                nmos_width: 1_000,
                pmos_width: 1_400,
            },
        });

        let nmos = ext_nmos();
        let pmos = ext_pmos();

        let pkg = Package {
            domain: "hi".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![nand2],
            ext_modules: vec![nmos, pmos],
        };

        let input = SimInput {
            pkg: Some(pkg),
            top: "nand2".to_string(),
            opts: None,
            an: vec![],
            ctrls: vec![],
        };

        vlsir::conv::save(&input, "hi.bin")?;
        Ok(())
    }
}
