use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use vlsir::circuit::{Instance, Package};
use vlsir::reference::To;
use vlsir::spice::SimInput;
use vlsir::Module;

pub const PRIMITIVE_DOMAIN: &str = "primitives";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[allow(dead_code)]
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
        writeln!(self.sink)?;

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
            writeln!(self.sink)?;

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

    fn emit_local_instance(&mut self, name: &str, _inst: &Instance) -> Result<()> {
        let module = self.modules.get(name).unwrap();
        for p in &module.ports {
            let _sig = p.signal.as_ref().unwrap();
        }
        Ok(())
    }

    fn emit_primitive(&mut self, _inst: &Instance) -> Result<()> {
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

    use vlsir::circuit::Package;

    use crate::decoder::{hierarchical_decoder, DecoderParams, DecoderTree};
    use crate::mos::{ext_nmos, ext_pmos};
    use crate::save_bin;
    use crate::NETLIST_FORMAT;

    use super::Result;

    #[test]
    fn test_netlist() -> Result<()> {
        let nmos = ext_nmos(NETLIST_FORMAT);
        let pmos = ext_pmos(NETLIST_FORMAT);

        let mut pkg = Package {
            domain: "sramgen_test_netlist".to_string(),
            desc: "Sramgen generated cells".to_string(),
            modules: vec![],
            ext_modules: vec![nmos, pmos],
        };

        let tree = DecoderTree::new(4);
        println!("tree: {:?}", &tree);
        let params = DecoderParams {
            tree,
            name: "hier_decode".to_string(),
            lch: 150,
        };

        let mut mods = hierarchical_decoder(params);

        pkg.modules.append(&mut mods);

        save_bin("decoder", pkg)?;
        Ok(())
    }
}
