pub mod spice;

use crate::error::Result;

pub trait NetlistBackend {
    fn subcircuit(&mut self, name: &str) -> Result<()>;
    fn end_subcircuit(&mut self) -> Result<()>;
    fn instance(
        &mut self,
        name: &str,
        terminals: &[&str],
        cell: &str,
        params: &[&str],
    ) -> Result<()>;
    fn temp_net(&mut self) -> String;
}
