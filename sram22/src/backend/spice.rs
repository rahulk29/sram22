use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::backend::NetlistBackend;
use crate::error::Result;

pub struct SpiceBackend {
    ofile: File,
    counter: u64,
}

impl SpiceBackend {
    pub fn new(ofile: impl AsRef<Path>) -> Self {
        Self {
            ofile: File::create(ofile).unwrap(),
            counter: 0,
        }
    }
}

impl Drop for SpiceBackend {
    fn drop(&mut self) {
        let _ = write!(self.ofile, ".end");
        let _ = self.ofile.sync_all();
    }
}

impl NetlistBackend for SpiceBackend {
    fn subcircuit(&mut self, name: &str) -> Result<()> {
        writeln!(self.ofile, ".subckt {}", name)?;
        Ok(())
    }

    fn end_subcircuit(&mut self) -> Result<()> {
        writeln!(self.ofile, ".ends")?;
        Ok(())
    }

    fn instance(
        &mut self,
        name: &str,
        terminals: &[&str],
        cell: &str,
        params: &[&str],
    ) -> Result<()> {
        write!(self.ofile, "X{}", name)?;

        for t in terminals {
            write!(self.ofile, " {}", *t)?;
        }

        write!(self.ofile, " {}", cell)?;

        for param in params {
            write!(self.ofile, " {}", *param)?;
        }

        writeln!(self.ofile)?;

        Ok(())
    }

    fn temp_net(&mut self) -> String {
        self.counter += 1;
        format!("int_{}", self.counter)
    }
}

impl SpiceBackend {
    pub fn lib(&mut self, lib_file: &str, lib_name: &str) -> Result<()> {
        writeln!(self.ofile, ".lib {} {}", lib_file, lib_name)?;
        Ok(())
    }

    pub fn title(&mut self, title: &str) -> Result<()> {
        writeln!(self.ofile, ".title {}", title)?;
        Ok(())
    }

    pub fn options(&mut self, options: &str) -> Result<()> {
        writeln!(self.ofile, ".options {}", options)?;
        Ok(())
    }

    pub fn comment(&mut self, comment: &str) -> Result<()> {
        writeln!(self.ofile, "* {}", comment)?;
        Ok(())
    }
}
